// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2021 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import React, { ReactNode } from "react";
import { Camera } from "../types";
import { Part, parsePart } from "./parser";
import * as api from "../api";
import Box from "@mui/material/Box";
import CircularProgress from "@mui/material/CircularProgress";
import Alert from "@mui/material/Alert";
import Typography from "@mui/material/Typography";
import useResizeObserver from "@react-hook/resize-observer";
import { fillAspect } from "../aspect";
import IconButton from "@mui/material/IconButton";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import ArrowBackIcon from "@mui/icons-material/ArrowBack";
import ArrowForwardIcon from "@mui/icons-material/ArrowForward";
import ZoomInIcon from "@mui/icons-material/ZoomIn";
import ZoomOutIcon from "@mui/icons-material/ZoomOut";

export const MediaSourceApi: typeof MediaSource | undefined =
  (self as any).ManagedMediaSource ?? self.MediaSource;

interface LiveCameraProps {
  mediaSourceApi: typeof MediaSource;
  camera: Camera | null;
  chooser: React.JSX.Element;
}

interface BufferStateClosed {
  state: "closed";
}
interface BufferStateOpen {
  state: "open";
  srcBuf: SourceBuffer;
  busy: boolean;
  mimeType: string;
  videoSampleEntryId: number;
}
interface BufferStateError {
  state: "error";
}
type BufferState = BufferStateClosed | BufferStateOpen | BufferStateError;

interface PlaybackStateNormal {
  state: "normal";
}
interface PlaybackStateWaiting {
  state: "waiting";
}
interface PlaybackStateError {
  state: "error";
  message: ReactNode;
}
type PlaybackState =
  | PlaybackStateNormal
  | PlaybackStateWaiting
  | PlaybackStateError;

interface DroppedMessage {
  type: "dropped";
  frames: number;
}
interface ErrorMessage {
  type: "error";
  message: string;
}
type Message = DroppedMessage | ErrorMessage;

class LiveCameraDriver {
  constructor(
    mediaSourceApi: typeof MediaSource,
    camera: Camera,
    setPlaybackState: (state: PlaybackState) => void,
    setAspect: (aspect: [number, number]) => void,
    video: HTMLVideoElement,
  ) {
    this.mediaSourceApi = mediaSourceApi;
    this.src = new mediaSourceApi();
    this.camera = camera;
    this.setPlaybackState = setPlaybackState;
    this.setAspect = setAspect;
    this.video = video;
    this.aborted = false;
    video.addEventListener("pause", this.videoPause);
    video.addEventListener("play", this.videoPlay);
    video.addEventListener("playing", this.videoPlaying);
    video.addEventListener("timeupdate", this.videoTimeUpdate);
    video.addEventListener("waiting", this.videoWaiting);
    this.src.addEventListener("sourceopen", this.onMediaSourceOpen);
    video["disableRemotePlayback"] = true;
    video.src = this.objectUrl = URL.createObjectURL(this.src);
    video.load();
  }

  unmount = () => {
    this.aborted = true;
    this.stopStream("unmount");
    const v = this.video;
    v.removeEventListener("pause", this.videoPause);
    v.removeEventListener("play", this.videoPlay);
    v.removeEventListener("playing", this.videoPlaying);
    v.removeEventListener("timeupdate", this.videoTimeUpdate);
    v.removeEventListener("waiting", this.videoWaiting);
    v.removeAttribute("src");
    URL.revokeObjectURL(this.objectUrl);
    v.load();
    this.buf = { state: "error" };
    this.queue = [];
  };

  onMediaSourceOpen = () => {
    this.startStream("sourceopen");
  };

  startStream = (reason: string) => {
    if (this.ws !== undefined) return;
    const mainStream = this.camera.streams.main;
    const subStream = this.camera.streams.sub;
    const hasMain = mainStream && mainStream.config?.mode !== "off";
    const hasSub = subStream && subStream.config?.mode !== "off";
    // Use MAIN stream for live view (continuous recording), SUB for AI detection only
    const streamType = hasMain ? "main" : hasSub ? "sub" : null;

    if (!streamType) {
      const details = [];
      if (!hasMain && !hasSub) details.push("No active streams configured");
      this.error(
        `No stream available: ${details.join(", ")}. Check Camera Settings`,
      );
      return;
    }
    const loc = window.location;
    const proto = loc.protocol === "https:" ? "wss" : "ws";
    const url = `${proto}://${loc.host}/api/cameras/${this.camera.uuid}/${streamType}/live.m4s`;
    this.ws = new WebSocket(url);
    this.ws.addEventListener("open", () => {});
    this.ws.addEventListener("close", (e) => {
      this.error(`Connection closed (${e.code})`);
    });
    this.ws.addEventListener("error", (e) => {
      this.error("Connection failed");
    });
    this.ws.addEventListener("message", this.onWsMessage);
  };

  error = (reason: string, extra?: ReactNode) => {
    this.stopStream(reason);
    this.buf = { state: "error" };
    this.queue = [];
    this.setPlaybackState({ state: "error", message: extra || reason });
  };

  tryAddInitSegment = async (id: number, buf: BufferStateOpen) => {
    const res = await api.init(id, {});
    if (res.status === "success") {
      this.setAspect(res.response.aspect);
      buf.srcBuf.appendBuffer(res.response.body);
    } else {
      this.error(`Init fetch error: ${res.status}`);
    }
  };

  onWsMessage = (e: MessageEvent<any>) => {
    if (typeof e.data === "string") {
      const message = JSON.parse(e.data) as Message;
      if (message.type === "error") {
        this.error(`Server: ${message.message}`);
      } else if (message.type === "dropped") {
        // ignore
      }
      return;
    }
    // Process blob immediately without chaining to avoid backlog
    this.processWsBlob(e.data as Blob);
  };

  processWsBlob = async (blob: Blob) => {
    if (
      this.aborted ||
      this.buf.state === "error" ||
      this.src.readyState === "closed" ||
      this.src.readyState === "ended"
    )
      return;
    try {
      const raw = new Uint8Array(await blob.arrayBuffer());
      const result = parsePart(raw);
      if (result.status === "error") {
        return;
      }
      const part = result.part;
      if (!this.mediaSourceApi.isTypeSupported(part.mimeType)) {
        return;
      }
      this.queue.push(part);
      if (this.buf.state === "closed") {
        const srcBuf = this.src.addSourceBuffer(part.mimeType);
        srcBuf.mode = "segments";
        srcBuf.addEventListener("updateend", this.bufUpdateEnd);
        this.buf = {
          state: "open",
          srcBuf,
          busy: true,
          mimeType: part.mimeType,
          videoSampleEntryId: part.videoSampleEntryId,
        };
        await this.tryAddInitSegment(part.videoSampleEntryId, this.buf);
      } else if (this.buf.state === "open") {
        await this.tryAppendPart(this.buf);
      }
    } catch (e) {
      console.error("Error processing blob:", e);
    }
  };

  bufUpdateEnd = () => {
    if (this.aborted || this.buf.state !== "open") return;
    this.buf.busy = false;
    this.tryTrimBuffer();
    this.tryAppendPart(this.buf);
  };

  tryAppendPart = async (buf: BufferStateOpen) => {
    if (
      this.aborted ||
      buf.busy ||
      this.src.readyState === "closed" ||
      this.src.readyState === "ended"
    )
      return;
    const part = this.queue.shift();
    if (part === undefined) return;
    if (buf.state !== "open") return;
    if (part.mimeType !== buf.mimeType)
      try {
        buf.srcBuf.changeType(part.mimeType);
      } catch {
        /* Ignore */
      }
    if (part.videoSampleEntryId !== buf.videoSampleEntryId) {
      buf.busy = true;
      buf.videoSampleEntryId = part.videoSampleEntryId;
      this.queue.unshift(part);
      await this.tryAddInitSegment(part.videoSampleEntryId, buf);
      return;
    }
    const b = buf.srcBuf.buffered;
    buf.srcBuf.timestampOffset = b.length > 0 ? b.end(b.length - 1) : 0;
    try {
      buf.srcBuf.appendBuffer(part.body);
      buf.busy = true;
    } catch {
      /* Ignore */
    }
  };

  tryTrimBuffer = () => {
    if (
      this.aborted ||
      this.buf.state !== "open" ||
      this.buf.busy ||
      this.buf.srcBuf.buffered.length === 0
    )
      return;
    const sb = this.buf.srcBuf;
    const firstTs = sb.buffered.start(0);
    if (firstTs < this.video.currentTime - 5) {
      try {
        sb.remove(firstTs, this.video.currentTime - 5);
        this.buf.busy = true;
      } catch {
        /* Ignore */
      }
    }
  };

  videoPause = () => {
    this.stopStream("pause");
  };
  videoPlay = () => {
    this.startStream("play");
  };
  videoPlaying = () => {
    if (this.buf.state !== "error") this.setPlaybackState({ state: "normal" });
  };
  videoWaiting = () => {
    if (this.buf.state !== "error") this.setPlaybackState({ state: "waiting" });
  };
  videoTimeUpdate = () => {};

  stopStream = (reason: string) => {
    if (this.ws === undefined) return;
    this.ws.close(1000);
    this.ws = undefined;
  };

  camera: Camera;
  setPlaybackState: (state: PlaybackState) => void;
  setAspect: (aspect: [number, number]) => void;
  video: HTMLVideoElement;
  mediaSourceApi: typeof MediaSource;
  src: MediaSource;
  buf: BufferState = { state: "closed" };
  queue: Part[] = [];
  objectUrl: string;
  ws?: WebSocket;
  aborted: boolean;
}

const LiveCamera = ({ mediaSourceApi, camera, chooser }: LiveCameraProps) => {
  const [aspect, setAspect] = React.useState<[number, number]>([16, 9]);
  const videoRef = React.useRef<HTMLVideoElement | null>(null);
  const boxRef = React.useRef<HTMLElement | null>(null);
  const [playbackState, setPlaybackState] = React.useState<PlaybackState>({
    state: "normal",
  });

  React.useLayoutEffect(() => {
    if (boxRef.current && videoRef.current)
      fillAspect(boxRef.current.getBoundingClientRect(), videoRef, aspect);
  }, [aspect, boxRef, videoRef]);

  useResizeObserver(boxRef as React.RefObject<HTMLElement>, (entry) => {
    if (videoRef.current) fillAspect(entry.contentRect, videoRef, aspect);
  });

  React.useEffect(() => {
    if (camera && videoRef.current) {
      const d = new LiveCameraDriver(
        mediaSourceApi,
        camera,
        setPlaybackState,
        setAspect,
        videoRef.current,
      );
      return () => d.unmount();
    }
  }, [camera, mediaSourceApi, videoRef]);

  const [currentTime, setCurrentTime] = React.useState(new Date());
  const [resolution, setResolution] = React.useState("");
  const [streamType, setStreamType] = React.useState<string>("");
  const [isRecording, setIsRecording] = React.useState(false);

  React.useEffect(() => {
    const timer = setInterval(() => setCurrentTime(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  React.useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    const updateRes = () => {
      if (video.videoWidth > 0) {
        setResolution(`${video.videoWidth}x${video.videoHeight}`);
      }
    };
    video.addEventListener("loadedmetadata", updateRes);
    video.addEventListener("resize", updateRes);
    return () => {
      video.removeEventListener("loadedmetadata", updateRes);
      video.removeEventListener("resize", updateRes);
    };
  }, [videoRef]);

  React.useEffect(() => {
    if (camera) {
      const mainStream = camera.streams.main;
      const subStream = camera.streams.sub;
      const hasMain = mainStream && mainStream.config?.mode !== "off";
      const hasSub = subStream && subStream.config?.mode !== "off";
      setStreamType(hasSub ? "SUB" : hasMain ? "MAIN" : "");
      setIsRecording(hasSub || hasMain ? true : false);
    } else {
      setIsRecording(false);
    }
  }, [camera]);

  const onPtz = async (x: number, y: number, zoom: number, stop = false) => {
    if (!camera) return;
    try {
      await api.ptzMove(camera.uuid, { x, y, zoom, stop }, {});
    } catch (e) {
      console.error("PTZ failed:", e);
    }
  };

  const hasOnvif = !!camera?.config?.onvifBaseUrl;

  return (
    <Box
      ref={boxRef}
      sx={{
        width: "100%",
        height: "100%",
        position: "relative",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        bgcolor: "#000",
        overflow: "hidden",
        "&:hover .ptz-controls": { opacity: 1 },
      }}
    >
      <video
        ref={videoRef}
        muted
        autoPlay
        playsInline
        style={{
          width: "100%",
          height: "100%",
          objectFit: "contain",
          zIndex: 1,
        }}
      />

      {/* PTZ Overlay Controls */}
      {hasOnvif && (
        <Box
          className="ptz-controls"
          sx={{
            position: "absolute",
            right: 15,
            top: "50%",
            transform: "translateY(-50%)",
            display: "flex",
            flexDirection: "column",
            gap: 1,
            zIndex: 10,
            opacity: 0,
            transition: "opacity 0.3s ease",
            bgcolor: "rgba(0,0,0,0.4)",
            p: 1,
            borderRadius: 2,
            border: "1px solid rgba(255,255,255,0.1)",
          }}
        >
          <Box
            sx={{
              display: "grid",
              gridTemplateColumns: "1fr 1fr 1fr",
              gap: 0.5,
            }}
          >
            <Box />
            <IconButton
              size="small"
              onMouseDown={() => onPtz(0, 1, 0)}
              onMouseUp={() => onPtz(0, 0, 0, true)}
              sx={{ color: "white" }}
            >
              <ArrowUpwardIcon fontSize="small" />
            </IconButton>
            <Box />

            <IconButton
              size="small"
              onMouseDown={() => onPtz(-1, 0, 0)}
              onMouseUp={() => onPtz(0, 0, 0, true)}
              sx={{ color: "white" }}
            >
              <ArrowBackIcon fontSize="small" />
            </IconButton>
            <Box sx={{ width: 32, height: 32 }} />
            <IconButton
              size="small"
              onMouseDown={() => onPtz(1, 0, 0)}
              onMouseUp={() => onPtz(0, 0, 0, true)}
              sx={{ color: "white" }}
            >
              <ArrowForwardIcon fontSize="small" />
            </IconButton>

            <Box />
            <IconButton
              size="small"
              onMouseDown={() => onPtz(0, -1, 0)}
              onMouseUp={() => onPtz(0, 0, 0, true)}
              sx={{ color: "white" }}
            >
              <ArrowDownwardIcon fontSize="small" />
            </IconButton>
            <Box />
          </Box>
          <Box
            sx={{
              display: "flex",
              justifyContent: "space-around",
              mt: 1,
              pt: 1,
              borderTop: "1px solid rgba(255,255,255,0.1)",
            }}
          >
            <IconButton
              size="small"
              onMouseDown={() => onPtz(0, 0, 1)}
              onMouseUp={() => onPtz(0, 0, 0, true)}
              sx={{ color: "white" }}
            >
              <ZoomInIcon fontSize="small" />
            </IconButton>
            <IconButton
              size="small"
              onMouseDown={() => onPtz(0, 0, -1)}
              onMouseUp={() => onPtz(0, 0, 0, true)}
              sx={{ color: "white" }}
            >
              <ZoomOutIcon fontSize="small" />
            </IconButton>
          </Box>
        </Box>
      )}

      {/* HUD: Camera Name + REC Indicator (Top Left) */}
      <Box
        sx={{
          position: "absolute",
          top: 12,
          left: 15,
          zIndex: 10,
          pointerEvents: "none",
          display: "flex",
          gap: 1,
        }}
      >
        {/* Camera Name Badge */}
        <Box
          sx={{
            bgcolor: "rgba(0,0,0,0.85)",
            px: 2,
            py: 1,
            borderRadius: 1.5,
            border: "1px solid rgba(255,255,255,0.12)",
            backdropFilter: "blur(8px)",
            boxShadow: "0 4px 12px rgba(0,0,0,0.4)",
          }}
        >
          <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
            <Box
              sx={{
                width: 8,
                height: 8,
                borderRadius: "50%",
                bgcolor: camera ? "#4caf50" : "#757575",
                boxShadow: camera ? "0 0 8px rgba(76, 175, 80, 0.6)" : "none",
                animation: camera ? "pulse 2s infinite" : "none",
                "@keyframes pulse": {
                  "0%": { opacity: 1 },
                  "50%": { opacity: 0.5 },
                  "100%": { opacity: 1 },
                },
              }}
            />
            <Typography
              variant="caption"
              sx={{
                fontWeight: 700,
                color: "#fff",
                fontFamily: "'Inter', 'system-ui', sans-serif",
                fontSize: "0.9rem",
                letterSpacing: 0.7,
                textTransform: "uppercase",
                whiteSpace: "nowrap",
              }}
            >
              {camera?.shortName || "NO CAMERA"}
            </Typography>
          </Box>
        </Box>
        {/* REC Indicator */}
        {isRecording && (
          <Box
            sx={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              bgcolor: "rgba(244, 67, 54, 0.9)",
              px: 1.5,
              py: 1,
              borderRadius: 1.5,
              border: "1px solid rgba(255,255,255,0.2)",
              backdropFilter: "blur(8px)",
              boxShadow: "0 2px 8px rgba(244, 67, 54, 0.4)",
              animation: "recPulse 1.5s infinite",
              "@keyframes recPulse": {
                "0%": { opacity: 1 },
                "50%": { opacity: 0.6 },
                "100%": { opacity: 1 },
              },
            }}
          >
            <Box
              sx={{
                width: 10,
                height: 10,
                borderRadius: "50%",
                bgcolor: "#fff",
                boxShadow: "0 0 6px rgba(255,255,255,0.8)",
              }}
            />
          </Box>
        )}
      </Box>

      {/* HUD: Date/Time & Resolution (Top Right) */}
      <Box
        sx={{
          position: "absolute",
          top: 12,
          right: 15,
          zIndex: 10,
          pointerEvents: "none",
          bgcolor: "rgba(0,0,0,0.85)",
          px: 2,
          py: 1,
          borderRadius: 1.5,
          border: "1px solid rgba(255,255,255,0.12)",
          backdropFilter: "blur(8px)",
          boxShadow: "0 4px 12px rgba(0,0,0,0.4)",
          minWidth: "160px",
        }}
      >
        {/* Date */}
        <Box
          sx={{
            color: "#bdbdbd",
            fontSize: "0.7rem",
            fontWeight: 600,
            fontFamily: "'Inter Mono', 'Monaco', monospace",
            letterSpacing: 0.5,
            mb: 0.5,
          }}
        >
          {currentTime
            .toLocaleDateString("es-ES", {
              weekday: "short",
              day: "2-digit",
              month: "2-digit",
              year: "numeric",
            })
            .toUpperCase()}
        </Box>
        {/* Time */}
        <Box
          sx={{
            color: "#4caf50",
            fontSize: "1rem",
            fontWeight: 700,
            fontFamily: "'Inter Mono', 'Monaco', monospace",
            letterSpacing: 1,
            mb: 1,
            textShadow: "0 0 10px rgba(76, 175, 80, 0.3)",
          }}
        >
          {currentTime.toLocaleTimeString("es-ES", { hour12: false })}
        </Box>
        {/* Stream & Resolution */}
        <Box
          sx={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            pt: 0.75,
            borderTop: "1px solid rgba(255,255,255,0.15)",
          }}
        >
          <Box
            sx={{
              display: "flex",
              alignItems: "center",
              gap: 0.5,
            }}
          >
            <Box
              sx={{
                width: 6,
                height: 6,
                borderRadius: "50%",
                bgcolor:
                  streamType === "SUB"
                    ? "#2196f3"
                    : streamType === "MAIN"
                      ? "#ff9800"
                      : "#757575",
              }}
            />
            <Box
              sx={{
                color:
                  streamType === "SUB"
                    ? "#2196f3"
                    : streamType === "MAIN"
                      ? "#ff9800"
                      : "#757575",
                fontSize: "0.65rem",
                fontWeight: 700,
                fontFamily: "monospace",
                letterSpacing: 0.5,
              }}
            >
              {streamType || "N/A"}
            </Box>
          </Box>
          <Box
            sx={{
              color: "#9e9e9e",
              fontSize: "0.65rem",
              fontWeight: 600,
              fontFamily: "monospace",
              letterSpacing: 0.5,
            }}
          >
            {resolution || (camera ? "LOAD" : "N/A")}
          </Box>
        </Box>
      </Box>

      {/* Connection Status (Bottom Left) */}
      {camera && (
        <Box
          sx={{
            position: "absolute",
            bottom: 15,
            left: 15,
            zIndex: 10,
            pointerEvents: "none",
            display: "flex",
            alignItems: "center",
            gap: 0.75,
          }}
        >
          <Box
            sx={{
              width: 6,
              height: 6,
              borderRadius: "50%",
              bgcolor:
                playbackState.state === "normal"
                  ? "#4caf50"
                  : playbackState.state === "waiting"
                    ? "#ff9800"
                    : "#f44336",
              boxShadow: "0 0 6px currentColor",
            }}
          />
          <Typography
            variant="caption"
            sx={{
              color:
                playbackState.state === "normal"
                  ? "#4caf50"
                  : playbackState.state === "waiting"
                    ? "#ff9800"
                    : "#f44336",
              fontSize: "0.65rem",
              fontWeight: 700,
              fontFamily: "monospace",
              letterSpacing: 0.5,
              textTransform: "uppercase",
            }}
          >
            {playbackState.state === "normal"
              ? "LIVE"
              : playbackState.state === "waiting"
                ? "BUFFER"
                : "ERROR"}
          </Typography>
        </Box>
      )}

      {/* Camera Selector (Bottom Center) */}
      <Box
        className="controls"
        sx={{
          position: "absolute",
          bottom: 15,
          left: "50%",
          transform: "translateX(-50%)",
          zIndex: 20,
          opacity: camera ? 0.2 : 1, // High visibility if no camera selected
          "&:hover": { opacity: 1 },
          transition: "opacity 0.4s ease-in-out",
          bgcolor: camera ? "transparent" : "rgba(255,255,255,0.05)",
          p: camera ? 0 : 4,
          borderRadius: 2,
          border: camera ? "none" : "1px dashed rgba(255,255,255,0.2)",
          textAlign: "center",
        }}
      >
        {!camera && (
          <Typography
            variant="caption"
            sx={{ display: "block", mb: 1, color: "rgba(255,255,255,0.5)" }}
          >
            Click to assign camera
          </Typography>
        )}
        {chooser}
      </Box>

      {/* Loading Spinner */}
      {playbackState.state === "waiting" && camera && (
        <Box
          sx={{
            position: "absolute",
            zIndex: 5,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            width: "100%",
            height: "100%",
            bgcolor: "rgba(0,0,0,0.4)",
          }}
        >
          <CircularProgress size={40} thickness={4} sx={{ color: "#fff" }} />
        </Box>
      )}

      {/* Error Message */}
      {playbackState.state === "error" && camera && (
        <Box
          sx={{ position: "absolute", bottom: 20, width: "80%", zIndex: 30 }}
        >
          <Alert
            severity="error"
            variant="filled"
            sx={{
              py: 0.5,
              borderRadius: 1,
              fontSize: "0.75rem",
              fontWeight: 600,
            }}
          >
            {playbackState.message}
          </Alert>
        </Box>
      )}
    </Box>
  );
};

export default LiveCamera;
