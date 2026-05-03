// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Copyright (C) 2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

use base::{err, Error};
use std::time::Duration;
use tracing::info;
use url::Url;

pub async fn discover_ssdp() -> Result<Vec<String>, Error> {
    info!("Starting SSDP discovery for ONVIF devices...");
    use socket2::Domain;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    let target = "urn:schemas-xmlsoap-org:device:NetworkVideoTransmitter:1";
    let marker = b"HTTP/1.1";

    let ips = ssdp_probe::ssdp_probe(
        marker,
        10,
        Duration::from_secs(2),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        ssdp_probe::SsdpMSearch {
            host: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250)), 1900),
            mx: 3,
            st: target,
            extra_lines: "",
        },
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250)), 1900),
        Domain::ipv4(),
    )
    .map_err(|e| {
        let b: base::ErrorBuilder = err!(Unknown, msg("ssdp search failed"), source(e.to_string()));
        let e: base::Error = b.into();
        e
    })?;

    Ok(ips.into_iter().map(|ip| ip.to_string()).collect())
}

pub async fn ptz_move(
    base_url: &Url,
    username: &str,
    password: &str,
    x: f32,
    y: f32,
    zoom: f32,
    stop: bool,
) -> Result<(), Error> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| {
            let b: base::ErrorBuilder = err!(Unknown, msg("fail create client"), source(e));
            let e: base::Error = b.into();
            e
        })?;

    // In a full implementation, we would first GetProfiles, then GetPTZConfiguration,
    // then ContinuousMove. For this prototype, we'll try to use a common ContinuousMove SOAP call.
    // This requires a ProfileToken. Most cameras have 'Profile_1' or similar.

    let soap_action = if stop {
        "http://www.onvif.org/ver20/ptz/wsdl/Stop"
    } else {
        "http://www.onvif.org/ver20/ptz/wsdl/ContinuousMove"
    };

    let ptz_url = base_url.join("onvif/ptz_service").map_err(|e| {
        let b: base::ErrorBuilder = err!(InvalidArgument, msg("bad ptz url"), source(e));
        let e: base::Error = b.into();
        e
    })?;

    // Extremely simplified SOAP body for prototype.
    // Real ONVIF requires Digest/WS-Security auth and correct namespaces.
    let body = if stop {
        format!(
            r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
                <s:Body>
                    <tptz:Stop>
                        <tptz:ProfileToken>Profile_1</tptz:ProfileToken>
                        <tptz:PanTilt>true</tptz:PanTilt>
                        <tptz:Zoom>true</tptz:Zoom>
                    </tptz:Stop>
                </s:Body>
            </s:Envelope>"#
        )
    } else {
        format!(
            r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
                <s:Body>
                    <tptz:ContinuousMove>
                        <tptz:ProfileToken>Profile_1</tptz:ProfileToken>
                        <tptz:Velocity>
                            <tt:PanTilt x="{}" y="{}" />
                            <tt:Zoom x="{}" />
                        </tptz:Velocity>
                    </tptz:ContinuousMove>
                </s:Body>
            </s:Envelope>"#,
            x, y, zoom
        )
    };

    let mut request = client
        .post(ptz_url)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .header("SOAPAction", soap_action)
        .body(body);

    if !username.is_empty() {
        request = request.basic_auth(username, Some(password));
    }

    let resp = request.send().await.map_err(|e| {
        let b: base::ErrorBuilder = err!(Unknown, msg("ptz request failed"), source(e));
        let e: base::Error = b.into();
        e
    })?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        let b: base::ErrorBuilder = err!(Unknown, msg("onvif ptz error: {}", err_text));
        let e: base::Error = b.into();
        return Err(e);
    }

    Ok(())
}
