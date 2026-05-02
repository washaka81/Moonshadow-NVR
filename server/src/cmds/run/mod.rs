// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Fork of Moonshadow NVR. Copyright (C) 2022 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// Copyright (C) 2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

use crate::streamer;
use crate::web;
use crate::web::accept::Listener;
use base::clock;
use base::err;
use base::FastHashMap;
use base::{bail, Error};
use bpaf::Bpaf;
use hyper::service::service_fn;
use itertools::Itertools;
use retina::client::SessionGroup;
use std::fmt::Write as _;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tracing::error;
use tracing::Instrument as _;
use tracing::{info, warn};

#[cfg(target_os = "linux")]
use libsystemd::daemon::{notify, NotifyState};

use self::config::ConfigFile;

pub mod config;

/// AI operation mode for intelligent surveillance.
#[derive(Bpaf, Debug, Clone, Copy)]
pub enum AiMode {
    /// Disable all AI processing.
    Off,
    /// Low resource usage: process 1 frame every 30 seconds.
    Low,
    /// Balanced: process 1 frame every 8 seconds (default).
    Medium,
    /// High performance: process 1 frame every 2 seconds.
    High,
    /// Automatically detect hardware capabilities and choose optimal mode.
    Auto,
}

impl std::str::FromStr for AiMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(AiMode::Off),
            "low" => Ok(AiMode::Low),
            "medium" => Ok(AiMode::Medium),
            "high" => Ok(AiMode::High),
            "auto" => Ok(AiMode::Auto),
            _ => Err(format!(
                "Invalid AI mode: '{}'. Valid modes: off, low, medium, high, auto",
                s
            )),
        }
    }
}

/// Runs the server, saving recordings and allowing web access.
#[derive(Bpaf, Debug)]
#[bpaf(command("run"))]
pub struct Args {
    /// Path to configuration file. See `ref/config.md` for config file documentation.
    #[bpaf(short, long, argument("PATH"), fallback("/etc/moonshadow-nvr.toml".into()), debug_fallback)]
    config: PathBuf,

    /// Opens the database in read-only mode and disables recording.
    /// Note this is incompatible with session authentication; consider adding
    /// a bind with `allowUnauthenticatedPermissions` to your config.
    read_only: bool,

    /// Path to YOLOv8 ONNX model for human and vehicle detection.
    #[bpaf(long, argument("PATH"))]
    model: Option<PathBuf>,

    /// Path to OSNet ONNX model for person re-identification.
    #[bpaf(long, argument("PATH"))]
    reid_model: Option<PathBuf>,

    /// Path to LPRNet ONNX model for license plate recognition.
    #[bpaf(long, argument("PATH"))]
    lpr_model: Option<PathBuf>,

    /// Path to YOLOv8-Face ONNX model for face detection.
    #[bpaf(long, argument("PATH"))]
    face_model: Option<PathBuf>,

    /// AI processing mode for intelligent surveillance.
    #[bpaf(long("ai-mode"), argument("MODE"), fallback(AiMode::Medium))]
    ai_mode: AiMode,

    /// Individual AI Features
    #[bpaf(long("enable-detection"), argument("BOOL"), fallback(true))]
    enable_detection: bool,

    #[bpaf(long("enable-lpr"), argument("BOOL"), fallback(true))]
    enable_lpr: bool,

    #[bpaf(long("enable-face"), argument("BOOL"), fallback(false))]
    enable_face: bool,

    #[bpaf(long("enable-heatmap"), argument("BOOL"), fallback(true))]
    enable_heatmap: bool,

    /// Enable hardware acceleration (OpenVINO) if available.
    #[bpaf(long("hardware-acceleration"), argument("BOOL"), fallback(true))]
    hardware_acceleration: bool,

    /// Automatically optimize settings for detected hardware capabilities.
    #[bpaf(long("optimize-for-device"), argument("BOOL"), fallback(true))]
    optimize_for_device: bool,
}

struct Flusher {
    channel: db::lifecycle::FlusherChannel,
    join: tokio::task::JoinHandle<()>,
}

#[cfg(target_os = "linux")]
fn get_preopened_sockets() -> Result<FastHashMap<String, Listener>, Error> {
    use libsystemd::activation::IsType as _;
    use std::os::fd::{FromRawFd, IntoRawFd};

    // `receive_descriptors_with_names` errors out if not running under systemd or not using socket
    // activation.
    if std::env::var_os("LISTEN_FDS").is_none() {
        info!("no LISTEN_FDs");
        return Ok(FastHashMap::default());
    }

    let sockets = libsystemd::activation::receive_descriptors_with_names(false)
        .map_err(|e| err!(Unknown, source(e), msg("unable to receive systemd sockets")))?;
    sockets
        .into_iter()
        .map(|(fd, name)| {
            if fd.is_unix() {
                // SAFETY: yes, it's a socket we own.
                let l = unsafe { std::os::unix::net::UnixListener::from_raw_fd(fd.into_raw_fd()) };
                l.set_nonblocking(true)?;
                Ok(Some((
                    name,
                    Listener::Unix(tokio::net::UnixListener::from_std(l)?),
                )))
            } else if fd.is_inet() {
                // SAFETY: yes, it's a socket we own.
                let l = unsafe { std::net::TcpListener::from_raw_fd(fd.into_raw_fd()) };
                l.set_nonblocking(true)?;
                Ok(Some((
                    name,
                    Listener::Tcp(tokio::net::TcpListener::from_std(l)?),
                )))
            } else {
                warn!("ignoring systemd socket {name:?} which is not unix or inet");
                Ok(None)
            }
        })
        .filter_map(Result::transpose)
        .collect()
}

#[cfg(not(target_os = "linux"))]
fn get_preopened_sockets() -> Result<FastHashMap<String, Listener>, Error> {
    Ok(FastHashMap::default())
}

fn read_config(path: &Path) -> Result<ConfigFile, Error> {
    let config = std::fs::read(path)?;
    let config = std::str::from_utf8(&config).map_err(|e| err!(InvalidArgument, source(e)))?;
    let config = toml::from_str(config).map_err(|e| err!(InvalidArgument, source(e)))?;
    Ok(config)
}

pub fn run(args: Args) -> Result<i32, Error> {
    let config = read_config(&args.config).map_err(|e| {
        err!(
            e,
            msg(
                "unable to load config file {}; see documentation in ref/config.md",
                &args.config.display(),
            ),
        )
    })?;

    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.enable_all();
    if let Some(worker_threads) = config.worker_threads {
        builder.worker_threads(worker_threads);
    }
    let rt = builder.build()?;
    let r = rt.block_on(async_run(args, &config));

    // tokio normally waits for all spawned tasks to complete, but:
    // * in the graceful shutdown path, we wait for specific tasks with logging.
    // * in the immediate shutdown path, we don't want to wait.
    rt.shutdown_background();

    r
}

async fn async_run(args: Args, config: &ConfigFile) -> Result<i32, Error> {
    let (shutdown_tx, shutdown_rx) = base::shutdown::channel();
    let mut shutdown_tx = Some(shutdown_tx);
    let (reload_tx, reload_rx) = tokio::sync::mpsc::channel(1);

    tokio::pin! {
        let int = signal(SignalKind::interrupt())?;
        let term = signal(SignalKind::terminate())?;
        let quit = signal(SignalKind::quit())?;
        let inner = inner(
            args.read_only,
            config,
            args.model,
            args.reid_model,
            args.lpr_model,
            args.face_model,
            args.ai_mode,
            args.enable_detection,
            args.enable_lpr,
            args.enable_face,
            args.enable_heatmap,
            args.hardware_acceleration,
            args.optimize_for_device,
            shutdown_rx,
            reload_rx,
            reload_tx,
        );
    }

    tokio::select! {
        _ = int.recv() => {
            info!("Received SIGINT; shutting down gracefully. \
                   Send another SIGINT or SIGTERM to shut down immediately.");
            shutdown_tx.take();
        },
        _ = term.recv() => {
            info!("Received SIGTERM; shutting down gracefully. \
                   Send another SIGINT or SIGTERM to shut down immediately.");
            shutdown_tx.take();
        },
        _ = quit.recv() => {
            #[cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(2),
                tokio::runtime::Handle::current().dump()
            ).await {
                Ok(dump) => {
                    info!("tokio task dump (on SIGQUIT) completed successfully");
                    for (i, t) in dump.tasks().iter().enumerate() {
                        info!("...task {i}: {trace}", trace=t.trace());
                    }
                },
                Err(_) => {
                    info!("tokio task dump (on SIGQUIT) timed out");
                }
            }
        }
        result = &mut inner => return result,
    }

    tokio::select! {
        _ = int.recv() => bail!(Cancelled, msg("immediate shutdown due to second signal (SIGINT)")),
        _ = term.recv() => bail!(Cancelled, msg("immediate shutdown due to second singal (SIGTERM)")),
        result = &mut inner => result,
    }
}

/// Makes a best-effort attempt to prepare a path for binding as a Unix-domain socket.
///
/// Binding to a Unix-domain socket fails with `EADDRINUSE` if the dirent already exists,
/// and the dirent isn't automatically deleted when the previous server closes. Clean up a
/// previous socket. As a defense against misconfiguration, make sure it actually is
/// a socket first.
///
/// This mechanism is inherently racy, but it's expected that the database has already
/// been locked.
fn prepare_unix_socket(p: &Path) {
    use nix::sys::stat::{stat, SFlag};
    let stat = match stat(p) {
        Err(_) => return,
        Ok(s) => s,
    };
    if !SFlag::from_bits_truncate(stat.st_mode).intersects(SFlag::S_IFSOCK) {
        return;
    }
    let _ = nix::unistd::unlink(p);
}

fn make_listener(
    addr: &config::AddressConfig,
    #[cfg_attr(not(target_os = "linux"), allow(unused))] preopened: &mut FastHashMap<
        String,
        Listener,
    >,
) -> Result<Listener, Error> {
    let sa: SocketAddr = match addr {
        config::AddressConfig::Ipv4(a) => (*a).into(),
        config::AddressConfig::Ipv6(a) => (*a).into(),
        config::AddressConfig::Unix(p) => {
            prepare_unix_socket(p);
            return Ok(Listener::Unix(tokio::net::UnixListener::bind(p).map_err(
                |e| err!(e, msg("unable bind Unix socket {}", p.display())),
            )?));
        }
        #[cfg(target_os = "linux")]
        config::AddressConfig::Systemd(n) => {
            return preopened.remove(n).ok_or_else(|| {
                err!(
                    NotFound,
                    msg(
                        "can't find systemd socket named {}; available sockets are: {}",
                        n,
                        preopened.keys().join(", ")
                    )
                )
                .build()
            });
        }
        #[cfg(not(target_os = "linux"))]
        config::AddressConfig::Systemd(_) => {
            bail!(Unimplemented, msg("systemd sockets are Linux-only"))
        }
    };

    // Go through std::net::TcpListener to avoid needing async. That's there for DNS resolution,
    // but it's unnecessary when starting from a SocketAddr.
    let listener = std::net::TcpListener::bind(sa)
        .map_err(|e| err!(e, msg("unable to bind TCP socket {sa}")))?;
    listener.set_nonblocking(true)?;
    Ok(Listener::Tcp(tokio::net::TcpListener::from_std(listener)?))
}

#[allow(clippy::too_many_arguments)]
async fn inner(
    read_only: bool,
    config: &ConfigFile,
    model_path: Option<PathBuf>,
    reid_model_path: Option<PathBuf>,
    lpr_model_path: Option<PathBuf>,
    face_model_path: Option<PathBuf>,
    ai_mode: AiMode,
    _enable_detection: bool,
    enable_lpr: bool,
    enable_face: bool,
    enable_heatmap: bool,
    hardware_acceleration: bool,
    optimize_for_device: bool,
    shutdown_rx: base::shutdown::Receiver,
    mut reload_rx: tokio::sync::mpsc::Receiver<()>,
    reload_tx: tokio::sync::mpsc::Sender<()>,
) -> Result<i32, Error> {
    let clocks = clock::RealClocks {};
    let (_db_dir, conn) = super::open_conn(
        &config.db_dir,
        if read_only {
            super::OpenMode::ReadOnly
        } else {
            super::OpenMode::ReadWrite
        },
    )?;
    let db = Arc::new(db::Database::new(clocks, conn, !read_only)?);
    info!("Database is loaded.");

    info!("=== Moonshadow NVR Configuration ===");
    let l = db.lock();
    let camera_count = l.cameras_by_id().len();
    info!("Cameras configured: {}", camera_count);

    for (id, camera) in l.cameras_by_id() {
        let id = *id;
        info!(
            "Camera #{}: {} - {}",
            id, camera.short_name, camera.config.description
        );
        let streams = l
            .streams_by_id()
            .values()
            .filter(|s| s.inner.lock().camera_id == id);
        for stream in streams {
            let stream_inner = stream.inner.lock();
            if let Some(url) = &stream_inner.config.url {
                info!(
                    "  Stream {:?}: {} ({}), retain: {} bytes",
                    stream_inner.type_,
                    url,
                    stream_inner.config.rtsp_transport,
                    stream_inner.config.retain_bytes
                );
            } else {
                info!("  Stream {:?}: disabled", stream_inner.type_);
            }
        }
    }
    drop(l);
    info!("=====================================");

    let flusher = if !read_only {
        let (channel, join) = db::lifecycle::start_flusher(db.clone());
        Some(Flusher { channel, join })
    } else {
        None
    };

    let dirs_to_open: Vec<_> = db
        .lock()
        .streams_by_id()
        .values()
        .filter_map(|s| s.inner.lock().sample_file_dir.as_ref().map(|d| d.id))
        .collect();
    db.open_sample_file_dirs(&dirs_to_open).await?;
    info!("Directories are opened.");

    // Auto-add recording directory if none exists
    {
        let l = db.lock();
        if l.sample_file_dirs_by_id().is_empty() {
            drop(l);
            let recordings_path = config.db_dir.parent().unwrap_or(Path::new("/")).to_path_buf();
            info!("Checking for recordings directory: {:?}", recordings_path);
            if recordings_path.exists() {
                info!("Attempting to auto-add recording directory: {:?}", recordings_path);
                match db.add_sample_file_dir(recordings_path).await {
                    Ok(dir_id) => {
                        info!("Added recording directory with ID: {}", dir_id);
                        // Assign this directory to all streams that don't have one
                        let streams_to_update: Vec<i32> = {
                            let l = db.lock();
                            l.streams_by_id()
                                .values()
                                .filter(|s| s.inner.lock().sample_file_dir.is_none())
                                .map(|s| s.inner.lock().id)
                                .collect()
                        };
                        for stream_id in streams_to_update {
                            let mut l = db.lock();
                            l.set_stream_sample_file_dir(stream_id, Some(dir_id))
                                .unwrap();
                        }
                    }
                    Err(e) => info!("Failed to auto-add directory: {}", e),
                }
            }
        }
    }

    db::lifecycle::abandon(&db).await?;
    db::lifecycle::initial_rotation(&db).await?;
    info!("Initial rotation is complete.");

    let mut detection_tx = None;
    if let Some(path) = model_path {
        let detector_ai_mode = match ai_mode {
            AiMode::Off => crate::detector::AiMode::Off,
            AiMode::Low => crate::detector::AiMode::Low,
            AiMode::Medium => crate::detector::AiMode::Medium,
            AiMode::High => crate::detector::AiMode::High,
            AiMode::Auto => crate::detector::AiMode::Auto,
        };

        let (vulkan_pre, ov_repair) = {
            let l = db.lock();
            let cfg = l.global_config();
            (cfg.vulkan_preprocessing, cfg.openvino_repair)
        };

        let detector = Arc::new(tokio::sync::Mutex::new(crate::detector::Detector::new(
            &path,
            reid_model_path.as_deref(),
            lpr_model_path.as_deref(),
            face_model_path.as_deref(),
            detector_ai_mode,
            hardware_acceleration,
            vulkan_pre,
            ov_repair,
            optimize_for_device,
        )?));
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        detection_tx = Some(tx);
        let worker = crate::detector::DetectionWorker::new(
            detector,
            rx,
            enable_lpr,
            enable_face,
            enable_heatmap,
            db.clocks(),
        );
        let db_clone = db.clone();
        tokio::task::Builder::new()
            .name("detection-worker")
            .spawn(async move {
                worker.run(db_clone).await;
            })
            .expect("spawn should succeed");
    }

    let zone = base::time::global_zone();
    let Some(time_zone_name) = zone.iana_name() else {
        bail!(
            Unknown,
            msg("unable to get IANA time zone name; check your $TZ, /etc/localtime, and /usr/share/zoneinfo/")
        );
    };
    info!("Resolved timezone: {}", &time_zone_name);

    // Manage streamers in a separate task to allow reloads.
    if !read_only {
        let db = db.clone();
        let shutdown_rx = shutdown_rx.clone();
        let detection_tx = detection_tx.clone();
        tokio::spawn(async move {
            loop {
                let mut streamers = tokio::task::JoinSet::new();
                let mut session_groups_by_camera: FastHashMap<
                    i32,
                    Arc<retina::client::SessionGroup>,
                > = FastHashMap::default();

                {
                    // Start up streams.
                    let l = db.lock();
                    let env = Box::leak(Box::new(streamer::Environment {
                        clocks: db.clocks(),
                        sample_entries: l.sample_entries().clone(),
                        opener: &crate::stream::OPENER,
                        shutdown_rx: shutdown_rx.clone(),
                        detection_tx: detection_tx.clone(),
                    }));
                    let streams_count = l.streams_by_id().len();
                    for (i, (_id, stream)) in l.streams_by_id().iter().enumerate() {
                        let locked = stream.inner.lock();
                        if locked.config.mode.is_empty() || locked.config.mode == "off" {
                            continue;
                        }
                        if locked.sample_file_dir.is_none() {
                            warn!(
                                "Stream {} has mode {:?} but has no sample file dir id",
                                locked.id, locked.config.mode
                            );
                            continue;
                        }
                        let camera = l.cameras_by_id().get(&locked.camera_id).unwrap();
                        let rotate_offset_sec =
                            streamer::ROTATE_INTERVAL_SEC * i as i64 / streams_count as i64;
                        let session_group = session_groups_by_camera
                            .entry(camera.id)
                            .or_insert_with(|| {
                                Arc::new(SessionGroup::default().named(camera.short_name.clone()))
                            })
                            .clone();
                        match streamer::Streamer::new(
                            env,
                            camera,
                            stream.clone(),
                            &locked,
                            session_group,
                            rotate_offset_sec,
                            streamer::ROTATE_INTERVAL_SEC,
                        ) {
                            Ok(mut streamer) => {
                                let span =
                                    tracing::info_span!("streamer", stream = streamer.short_name());
                                streamers
                                    .build_task()
                                    .name(&format!("s-{}", streamer.short_name()))
                                    .spawn(
                                        async move {
                                            info!("starting");
                                            streamer.run().await;
                                            info!("ending");
                                        }
                                        .instrument(span),
                                    )
                                    .expect("creating streamer task should succeed");
                            }
                            Err(e) => {
                                error!("Failed to create streamer for {}: {}", locked.id, e);
                            }
                        }
                    }
                }

                tokio::select! {
                    msg = reload_rx.recv() => {
                        if msg.is_none() { break; }
                        info!("Reload signal received, restarting streamers...");
                        streamers.abort_all();
                        while streamers.join_next().await.is_some() {}
                        if let Err(e) = db.reload() {
                            error!("Failed to reload database: {}", e);
                        }
                        // Loop continues and restarts streamers with new config
                    }
                    _ = shutdown_rx.as_future() => {
                        info!("Shutdown signal received, stopping streamers...");
                        streamers.abort_all();
                        while streamers.join_next().await.is_some() {}
                        break;
                    }
                    Some(result) = streamers.join_next() => {
                        if let Err(e) = result {
                            error!("Streamer task panicked: {}", e);
                        }
                    }
                }
            }
        });
    }

    // Start the web interface(s).
    let own_euid = nix::unistd::Uid::effective();
    let mut preopened = get_preopened_sockets()?;
    for bind in &config.binds {
        let svc = Arc::new(web::Service::new(web::Config {
            db: db.clone(),
            ui_dir: Some(&config.ui_dir),
            allow_unauthenticated_permissions: bind
                .allow_unauthenticated_permissions
                .clone()
                .map(db::Permissions::from),
            trust_forward_hdrs: bind.trust_forward_headers,
            time_zone_name: time_zone_name.to_owned(),
            privileged_unix_uid: bind.own_uid_is_privileged.then_some(own_euid),
            reload_tx: reload_tx.clone(),
        })?);
        let mut listener = make_listener(&bind.address, &mut preopened)?;
        let addr = bind.address.clone();
        tokio::task::Builder::new()
            .name(&format!("listener-{addr}"))
            .spawn(async move {
                loop {
                    let conn = match listener.accept().await {
                        Ok(c) => c,
                        Err(e) => {
                            error!(err = %e, listener = %addr, "accept failed; will retry in 1 sec");
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            continue;
                        }
                    };
                    let svc = Arc::clone(&svc);
                    let conn_data = *conn.data();
                    let io = hyper_util::rt::TokioIo::new(conn);
                    let svc = Arc::clone(&svc);
                    let svc_fn = service_fn(move |req| Arc::clone(&svc).serve(req, conn_data));

                    let mut task_name = format!("httpconn-{addr}");
                    if let Some(addr) = conn_data.client_addr.as_ref() {
                        let _ = write!(&mut task_name, "-{addr}");
                    }
                    tokio::task::Builder::new()
                        .name(&task_name)
                        .spawn(hyper::server::conn::http1::Builder::new()
                            .serve_connection(io, svc_fn)
                            .with_upgrades(),
                        ).expect("spawn should succeed");
                }
            }).expect("spawn should succeed");
    }
    if !preopened.is_empty() {
        warn!(
            "ignoring systemd sockets not referenced in config: {}",
            preopened.keys().join(", ")
        );
    }

    #[cfg(target_os = "linux")]
    {
        if let Err(err) = notify(false, &[NotifyState::Ready]) {
            tracing::warn!(%err, "unable to notify systemd on ready");
        }
    }

    info!("Ready to serve HTTP requests");
    shutdown_rx.as_future().await;

    #[cfg(target_os = "linux")]
    {
        if let Err(err) = notify(false, &[NotifyState::Stopping]) {
            tracing::warn!(%err, "unable to notify systemd on stopping");
        }
    }

    info!("Shutting down directory pools and flusher.");
    if let Some(flusher) = flusher {
        let dirs: Vec<_> = db.lock().sample_file_dirs_by_id().keys().cloned().collect();
        db.close_sample_file_dirs(&dirs).await?;
        drop(flusher.channel);
        flusher.join.await.unwrap();
    }

    info!("Exiting.");
    Ok(0)
}
