// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2025 Moonshadow NVR Contributors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import {
  FormContainer,
  TextFieldElement,
  CheckboxElement,
  RadioButtonGroup,
  useFormContext,
} from "react-hook-form-mui";
import Button from "@mui/material/Button";
import Dialog from "@mui/material/Dialog";
import DialogActions from "@mui/material/DialogActions";
import DialogContent from "@mui/material/DialogContent";
import DialogTitle from "@mui/material/DialogTitle";
import * as api from "../api";
import { Camera } from "../types";
import Box from "@mui/material/Box";
import Chip from "@mui/material/Chip";
import React, { useEffect, useState } from "react";
import { useSnackbars } from "../snackbars";
import Typography from "@mui/material/Typography";
import Divider from "@mui/material/Divider";
import Grid from "@mui/material/Grid";
import FormControl from "@mui/material/FormControl";
import InputAdornment from "@mui/material/InputAdornment";
import TravelExploreIcon from "@mui/icons-material/TravelExplore";
import VideoSettingsIcon from "@mui/icons-material/VideoSettings";
import { LoadingButton } from "@mui/lab";

interface Props {
  prior: Camera | null;
  csrf?: string;
  onClose: () => void;
  refetch: () => void;
}

interface FormData {
  shortName: string;
  description: string;
  onvifBaseUrl: string;
  username: string;
  password: string;
  streams: {
    enabled: boolean;
    url: string;
    rtspTransport: "tcp" | "udp" | "";
    retainBytesGb: number;
    detectedCodec?: string;
  }[];
}

const STREAM_NAMES = ["Main", "Sub", "Ext"];

const Inner = ({ csrf }: { csrf?: string }) => {
  const snackbars = useSnackbars();
  const { watch, setValue, getValues } = useFormContext<FormData>();
  const onvifBaseUrl = watch("onvifBaseUrl");
  const [autodetecting, setAutodetecting] = useState(false);
  const [probing, setProbing] = useState<number | null>(null);

  const onAutodetect = async () => {
    let ip = onvifBaseUrl;
    try {
      const url = new URL(ip);
      ip = url.hostname;
    } catch {
      // Not a valid URL, assume it's an IP.
    }
    
    // Fallback to "discovery" if IP is empty
    const targetIp = ip || "discovery";

    setAutodetecting(true);
    const { username, password } = getValues();
    const resp = await api.autodetectCamera(
      {
        csrf,
        ip: targetIp,
        username,
        password,
      },
      {},
    );
    setAutodetecting(false);

    switch (resp.status) {
      case "aborted":
        break;
      case "error":
        snackbars.enqueue({
          message: "Autodetect failed: " + resp.message,
        });
        break;
      case "success":
        snackbars.enqueue({
          message: `Autodetect successful! Found: ${resp.response.mainCodec || "Unknown"} / ${resp.response.subCodec || "Unknown"}`,
          severity: "success",
        });
        if (!onvifBaseUrl && targetIp !== "discovery") {
           setValue("onvifBaseUrl", `http://${targetIp}`, { shouldDirty: true });
        }
        if (resp.response.mainUrl) {
          setValue("streams.0.url", resp.response.mainUrl, {
            shouldDirty: true,
          });
          setValue("streams.0.enabled", true, { shouldDirty: true });
          if (resp.response.mainCodec) {
            setValue("streams.0.detectedCodec", resp.response.mainCodec);
          }
        }
        if (resp.response.subUrl) {
          setValue("streams.1.url", resp.response.subUrl, {
            shouldDirty: true,
          });
          setValue("streams.1.enabled", true, { shouldDirty: true });
          if (resp.response.subCodec) {
            setValue("streams.1.detectedCodec", resp.response.subCodec);
          }
        }
        break;
    }
  };

  const onProbe = async (index: number) => {
    const url = getValues(`streams.${index}.url`);
    if (!url) return;

    setProbing(index);
    const resp = await api.streamProbe({ csrf, url }, {});
    setProbing(null);

    if (resp.status === "success") {
      setValue(
        `streams.${index}.detectedCodec`,
        resp.response.codec || "Unknown",
      );
      snackbars.enqueue({
        message: `Probe successful! Codec: ${resp.response.codec || "Unknown"}`,
        severity: "success",
      });
    } else if (resp.status === "error") {
      snackbars.enqueue({
        message: `Probe failed: ${resp.message}`,
        severity: "error",
      });
    }
  };

  return (
    <>
      <DialogContent>
        <Grid container spacing={2}>
          <Grid size={12}>
            <Typography variant="h6">General</Typography>
          </Grid>
          <Grid size={{ xs: 12, sm: 6 }}>
            <TextFieldElement
              name="shortName"
              label="Short Name"
              required
              fullWidth
              variant="filled"
            />
          </Grid>
          <Grid size={{ xs: 12, sm: 6 }}>
            <FormControl variant="filled" fullWidth>
              <TextFieldElement
                name="onvifBaseUrl"
                label="ONVIF Base URL / IP Address"
                fullWidth
                variant="filled"
                placeholder="http://192.168.1.100:80"
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      <LoadingButton
                        aria-label="autodetect"
                        onClick={onAutodetect}
                        loading={autodetecting}
                        title="Autodetect Streams"
                      >
                        <TravelExploreIcon />
                      </LoadingButton>
                    </InputAdornment>
                  ),
                }}
              />
            </FormControl>
          </Grid>
          <Grid size={12}>
            <TextFieldElement
              name="description"
              label="Description"
              fullWidth
              multiline
              rows={2}
              variant="filled"
            />
          </Grid>
          <Grid size={{ xs: 12, sm: 6 }}>
            <TextFieldElement
              name="username"
              label="Username"
              fullWidth
              variant="filled"
              autoComplete="off"
            />
          </Grid>
          <Grid size={{ xs: 12, sm: 6 }}>
            <TextFieldElement
              name="password"
              label="Password"
              type="password"
              fullWidth
              variant="filled"
              autoComplete="off"
            />
          </Grid>

          {STREAM_NAMES.map((name, i) => (
            <React.Fragment key={name}>
              <Grid size={12}>
                <Box sx={{ mt: 2, mb: 1 }}>
                  <Divider>
                    <Typography variant="h6">{name} Stream</Typography>
                  </Divider>
                </Box>
              </Grid>
              <Grid
                size={12}
                sx={{ display: "flex", alignItems: "center", gap: 2 }}
              >
                <CheckboxElement
                  name={`streams.${i}.enabled`}
                  label="Enabled"
                />
                {watch(`streams.${i}.detectedCodec`) && (
                  <Chip
                    label={`Detected: ${watch(`streams.${i}.detectedCodec`)}`}
                    size="small"
                    color="primary"
                    variant="outlined"
                  />
                )}
              </Grid>
              <Grid size={12}>
                <TextFieldElement
                  name={`streams.${i}.url`}
                  label="RTSP URL"
                  fullWidth
                  variant="filled"
                  placeholder="rtsp://192.168.1.100:554/live"
                  InputProps={{
                    endAdornment: (
                      <InputAdornment position="end">
                        <LoadingButton
                          aria-label="probe"
                          onClick={() => onProbe(i)}
                          loading={probing === i}
                          title="Probe Codec"
                        >
                          <VideoSettingsIcon />
                        </LoadingButton>
                      </InputAdornment>
                    ),
                  }}
                />
              </Grid>
              <Grid size={{ xs: 12, sm: 6 }}>
                <RadioButtonGroup
                  name={`streams.${i}.rtspTransport`}
                  label="Transport"
                  options={[
                    { id: "tcp", label: "TCP" },
                    { id: "udp", label: "UDP" },
                  ]}
                  row
                />
              </Grid>
              <Grid size={{ xs: 12, sm: 6 }}>
                <TextFieldElement
                  name={`streams.${i}.retainBytesGb`}
                  label="Retention (GB)"
                  type="number"
                  fullWidth
                  variant="filled"
                />
              </Grid>
            </React.Fragment>
          ))}
        </Grid>
      </DialogContent>
    </>
  );
};

export default function AddEditDialog({
  prior,
  csrf,
  onClose,
  refetch,
}: Props): React.JSX.Element {
  const [req, setReq] = useState<api.CameraSubset | undefined>();
  const snackbars = useSnackbars();

  useEffect(() => {
    const abort = new AbortController();
    const send = async (camera: api.CameraSubset, signal: AbortSignal) => {
      const resp = prior
        ? await api.patchCamera(
            prior.uuid,
            {
              csrf: csrf,
              update: camera,
            },
            { signal },
          )
        : await api.postCamera(
            {
              csrf: csrf,
              camera: camera,
            },
            { signal },
          );
      setReq(undefined);
      switch (resp.status) {
        case "aborted":
          break;
        case "error":
          snackbars.enqueue({
            message: "Request failed: " + resp.message,
          });
          break;
        case "success":
          refetch();
          onClose();
          break;
      }
    };
    if (req !== undefined) {
      send(req, abort.signal);
    }
    return () => {
      abort.abort();
    };
  }, [prior, req, csrf, snackbars, onClose, refetch]);

  const onSuccess = (data: FormData) => {
    setReq({
      shortName: data.shortName,
      description: data.description,
      onvifBaseUrl: data.onvifBaseUrl || undefined,
      username: data.username,
      password: data.password,
      streams: data.streams.map((s) => ({
        enabled: s.enabled,
        url: s.url || undefined,
        rtspTransport: s.rtspTransport,
        retainBytes: Math.round(s.retainBytesGb * 1024 * 1024 * 1024),
      })),
    });
  };

  const defaultValues: FormData = {
    shortName: prior?.shortName ?? "",
    description: prior?.description ?? "",
    onvifBaseUrl: prior?.config?.onvifBaseUrl ?? "",
    username: prior?.config?.username ?? "",
    password: prior?.config?.password ?? "",
    streams: [0, 1, 2].map((i) => {
      const s = prior?.streams[STREAM_NAMES[i].toLowerCase() as api.StreamType];
      return {
        enabled: s !== undefined,
        url: s?.config?.url ?? "",
        rtspTransport: (s?.config?.rtspTransport as any) ?? "tcp",
        retainBytesGb:
          (s?.config?.retainBytes ?? 50 * 1024 * 1024 * 1024) /
          (1024 * 1024 * 1024),
      };
    }),
  };

  return (
    <Dialog open={true} maxWidth="md" fullWidth>
      <DialogTitle>
        {prior === null ? "Add camera" : `Edit camera ${prior.shortName}`}
      </DialogTitle>
      <FormContainer<FormData>
        defaultValues={defaultValues}
        onSuccess={onSuccess}
      >
        <Inner csrf={csrf} />
        <DialogActions>
          <Button onClick={onClose}>Cancel</Button>
          <Button
            loading={req !== undefined}
            color="primary"
            variant="contained"
            type="submit"
          >
            {prior === null ? "Add" : "Edit"}
          </Button>
        </DialogActions>
      </FormContainer>
    </Dialog>
  );
}
