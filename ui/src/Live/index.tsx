// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2021 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import Container from "@mui/material/Container";
import ErrorIcon from "@mui/icons-material/Error";
import { Camera } from "../types";
import LiveCamera, { MediaSourceApi } from "./LiveCamera";
import Multiview, {
  MultiviewChooser,
  MAX_CAMERAS,
  selectedReducer,
  getInitialSelected,
} from "./Multiview"; // Import moved exports
import { FrameProps } from "../App";
import { useSearchParams } from "react-router-dom";
import React, { useEffect, useState, useReducer } from "react"; // Added useReducer and React
import Drawer from "@mui/material/Drawer"; // Added for the drawer
import Box from "@mui/material/Box"; // Added for the drawer
import List from "@mui/material/List"; // Added for the drawer
import ListItem from "@mui/material/ListItem"; // Added for the drawer
import ListItemText from "@mui/material/ListItemText"; // Added for the drawer
import ListItemIcon from "@mui/material/ListItemIcon"; // Added for the drawer
import Fab from "@mui/material/Fab"; // Added for the floating action button
import Tooltip from "@mui/material/Tooltip"; // Added for tooltips
import GridViewIcon from "@mui/icons-material/GridView"; // Icon for mosaic organizer
import CloseIcon from "@mui/icons-material/Close"; // Icon for closing drawer
import CameraAltIcon from "@mui/icons-material/CameraAlt"; // Icon for draggable cameras
import IconButton from "@mui/material/IconButton"; // For drawer close button
import Button from "@mui/material/Button"; // For Auto-fill / Clear buttons
import Divider from "@mui/material/Divider"; // For divider in drawer
import Typography from "@mui/material/Typography";

export interface LiveProps {
  cameras: Camera[];
  Frame: (props: FrameProps) => React.JSX.Element;
}

const Live = ({ cameras, Frame }: LiveProps) => {
  const [searchParams, setSearchParams] = useSearchParams();
  const [isDrawerOpen, setIsDrawerOpen] = useState(false); // State for drawer visibility

  const [selected, updateSelected] = useReducer(
    selectedReducer,
    getInitialSelected(searchParams, cameras),
  );

  // Effect to keep localStorage and URL in sync with the lifted 'selected' state
  useEffect(() => {
    localStorage.setItem("camsSelected", JSON.stringify(selected));
    const newParams = new URLSearchParams(window.location.search.split("?")[1]);
    newParams.set("cams", JSON.stringify(selected));
    // Use window.history to update URL without triggering a full React Router re-render loop if possible,
    // or just ensure we don't depend on searchParams here.
    setSearchParams(newParams, { replace: true });
  }, [selected, setSearchParams]);

  // Auto-detect layout based on selected cameras count
  const selectedCount = selected.filter(
    (c): c is number => c !== null && c !== undefined,
  ).length;
  const getAutoLayoutIndex = () => {
    if (selectedCount <= 1) return 0; // solo
    if (selectedCount === 2) return 1; // dual
    if (selectedCount === 3) return 3; // 2x2
    if (selectedCount === 4) return 3; // 2x2
    if (selectedCount >= 5 && selectedCount <= 6) return 6; // 3x2
    if (selectedCount >= 7 && selectedCount <= 9) return 6; // 3x3
    return 6; // default to 3x3
  };

  const [multiviewLayoutIndex, setMultiviewLayoutIndex] = useState(() => {
    // Priority: URL param > localStorage (validated) > auto-detect
    if (searchParams.has("layout")) {
      const urlValue = Number.parseInt(searchParams.get("layout") || "0", 10);
      if (!isNaN(urlValue)) return urlValue;
    }
    try {
      const stored = localStorage.getItem("multiviewLayoutIndex");
      if (stored) {
        const parsed = Number.parseInt(stored, 10);
        if (!isNaN(parsed)) return parsed;
      }
    } catch (e) {
      localStorage.removeItem("multiviewLayoutIndex");
    }
    return getAutoLayoutIndex();
  });

  useEffect(() => {
    if (searchParams.has("layout"))
      localStorage.setItem(
        "multiviewLayoutIndex",
        searchParams.get("layout") || "0",
      );
  }, [searchParams]);

  const mediaSourceApi = MediaSourceApi;
  if (mediaSourceApi === undefined) {
    return (
      <Frame>
        <Container>
          <ErrorIcon
            sx={{
              float: "left",
              color: "secondary.main",
              marginRight: "1em",
            }}
          />
          Live view doesn't work yet on your browser. See{" "}
          <a href="https://github.com/washaka81/Moonshadow-NVR/issues/121">
            #121
          </a>
          .
        </Container>
      </Frame>
    );
  }

  const handleAutoFill = () => {
    const newSelected = Array(MAX_CAMERAS).fill(null);
    const camerasToSelect = Math.min(cameras.length, MAX_CAMERAS);
    for (let i = 0; i < camerasToSelect; i++) {
      newSelected[i] = i;
    }
    // Manually dispatch multiple ops or create a new reducer action for batch update
    newSelected.forEach((camIdx, idx) => {
      updateSelected({ selectedIndex: idx, cameraIndex: camIdx });
    });
  };

  const handleClearAll = () => {
    for (let i = 0; i < MAX_CAMERAS; i++) {
      updateSelected({ selectedIndex: i, cameraIndex: null });
    }
  };

  return (
    <Frame
      activityMenuPart={
        <>
          <MultiviewChooser
            layoutIndex={multiviewLayoutIndex}
            onChoice={(value) => {
              setMultiviewLayoutIndex(value);
              setSearchParams({ layout: value.toString() });
            }}
          />
          <Tooltip title="Organize Mosaic" arrow>
            <Fab
              color="primary"
              size="small"
              onClick={() => setIsDrawerOpen(true)}
              sx={{ ml: 1, boxShadow: "none" }}
            >
              <GridViewIcon fontSize="small" />
            </Fab>
          </Tooltip>
        </>
      }
    >
      <Multiview
        layoutIndex={multiviewLayoutIndex}
        cameras={cameras}
        selected={selected}
        updateSelected={updateSelected}
        renderCamera={(camera: Camera | null, chooser: React.JSX.Element) => (
          <LiveCamera
            mediaSourceApi={mediaSourceApi}
            camera={camera}
            chooser={chooser}
          />
        )}
      />

      <Drawer
        anchor="right"
        open={isDrawerOpen}
        onClose={() => setIsDrawerOpen(false)}
        PaperProps={{
          sx: {
            width: 300,
            bgcolor: "background.paper",
            borderLeft: "1px solid rgba(255,255,255,0.1)",
          },
        }}
      >
        <Box
          sx={{
            p: 2,
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
          }}
        >
          <Typography variant="h6">Organize Mosaic</Typography>
          <IconButton onClick={() => setIsDrawerOpen(false)} size="small">
            <CloseIcon />
          </IconButton>
        </Box>
        <Divider />
        <Box
          sx={{ p: 2, display: "flex", gap: 1, justifyContent: "space-around" }}
        >
          <Button variant="outlined" size="small" onClick={handleAutoFill}>
            Auto-fill
          </Button>
          <Button
            variant="outlined"
            size="small"
            color="error"
            onClick={handleClearAll}
          >
            Clear All
          </Button>
        </Box>
        <Divider />
        <List>
          {cameras.map((camera, index) => (
            <ListItem
              key={camera.uuid}
              draggable
              onDragStart={(e) => {
                e.dataTransfer.setData("cameraIndex", index.toString());
              }}
              sx={{
                "&:hover": { bgcolor: "rgba(255,255,255,0.05)" },
                cursor: "grab",
              }}
            >
              <ListItemIcon>
                <CameraAltIcon />
              </ListItemIcon>
              <ListItemText primary={camera.shortName} />
            </ListItem>
          ))}
        </List>
      </Drawer>
    </Frame>
  );
};

export default Live;
