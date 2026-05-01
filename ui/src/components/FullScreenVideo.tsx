// This file is part of Moonshadow NVR.
// Copyright (C) 2025 The Moonshadow NVR Authors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

import React, { useState, useEffect, useLayoutEffect } from "react";
import Box from "@mui/material/Box";
import IconButton from "@mui/material/IconButton";
import Button from "@mui/material/Button";
import DownloadIcon from "@mui/icons-material/Download";
import CloseIcon from "@mui/icons-material/Close";
import useResizeObserver from "@react-hook/resize-observer";
import { fillAspect } from "../aspect";

export interface FullScreenVideoProps {
  src: string;
  aspect?: [number, number];
  onClose: () => void;
  initialSpeed?: number;
  filename?: string;
}

export const FullScreenVideo = ({
  src,
  aspect = [16, 9],
  onClose,
  initialSpeed = 1,
  filename = "recording.mp4",
}: FullScreenVideoProps) => {
  const boxRef = React.useRef<HTMLElement | null>(null);
  const videoRef = React.useRef<HTMLVideoElement | null>(null);
  const [speed, setSpeed] = useState(initialSpeed);

  useLayoutEffect(() => {
    if (boxRef.current) {
      fillAspect(boxRef.current.getBoundingClientRect(), videoRef, aspect);
    }
  }, [aspect]);

  useEffect(() => {
    if (videoRef.current) {
      videoRef.current.playbackRate = speed;
    }
  }, [speed]);

  useResizeObserver(
    boxRef as React.RefObject<HTMLElement>,
    (entry: ResizeObserverEntry) => {
      fillAspect(entry.contentRect, videoRef, aspect);
    },
  );

  const toggleSpeed = () => {
    const nextSpeed = speed >= 16 ? 1 : speed * 2;
    setSpeed(nextSpeed);
    if (videoRef.current) {
      videoRef.current.playbackRate = nextSpeed;
    }
  };

  return (
    <Box
      ref={boxRef}
      tabIndex={-1}
      sx={{
        width: "100vw",
        height: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        bgcolor: "#000",
        position: "relative",
        "& video": { pointerEvents: "auto", outline: "none" },
      }}
    >
      <video ref={videoRef} controls preload="auto" autoPlay src={src} />

      {/* Top Left Close Button */}
      <IconButton
        onClick={onClose}
        sx={{
          position: "absolute",
          top: 20,
          left: 20,
          zIndex: 110,
          color: "rgba(255,255,255,0.7)",
          bgcolor: "rgba(0,0,0,0.3)",
          "&:hover": { bgcolor: "rgba(0,0,0,0.5)", color: "#fff" },
        }}
      >
        <CloseIcon />
      </IconButton>

      <Box
        sx={{
          position: "absolute",
          top: 20,
          right: 20,
          zIndex: 100,
          display: "flex",
          flexDirection: "column",
          gap: 1,
        }}
      >
        <Button
          variant="contained"
          size="small"
          onClick={toggleSpeed}
          sx={{
            minWidth: "60px",
            bgcolor: "rgba(255, 255, 255, 0.2)",
            backdropFilter: "blur(5px)",
            color: "white",
            "&:hover": { bgcolor: "rgba(255, 255, 255, 0.3)" },
          }}
        >
          {speed}x
        </Button>
        <Button
          variant="contained"
          size="small"
          component="a"
          href={src}
          download={filename}
          sx={{
            minWidth: "60px",
            bgcolor: "rgba(255, 255, 255, 0.2)",
            backdropFilter: "blur(5px)",
            color: "white",
            "&:hover": { bgcolor: "rgba(255, 255, 255, 0.3)" },
          }}
        >
          <DownloadIcon />
        </Button>
      </Box>
    </Box>
  );
};
