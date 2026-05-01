// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2021 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import Box from "@mui/material/Box";
import Select, { SelectChangeEvent } from "@mui/material/Select";
import MenuItem from "@mui/material/MenuItem";
import React from "react";
import { Camera } from "../types";

export interface Layout {
  className: string;
  cameras: number;
  name: string;
}

export const LAYOUTS: Layout[] = [
  { className: "solo", cameras: 1, name: "1" },
  { className: "dual", cameras: 2, name: "2" },
  { className: "main-plus-five", cameras: 6, name: "Main + 5" },
  { className: "two-by-two", cameras: 4, name: "2x2" },
  { className: "two-by-three", cameras: 6, name: "2x3" },
  { className: "three-by-two", cameras: 6, name: "3x2" },
  { className: "three-by-three", cameras: 9, name: "3x3" },
];

export interface MultiviewProps {
  cameras: Camera[];
  layoutIndex: number;
  selected: Array<number | null>;
  updateSelected: (op: SelectOp) => void;
  renderCamera: (
    camera: Camera | null,
    chooser: React.JSX.Element,
  ) => React.JSX.Element;
}

export interface MultiviewChooserProps {
  layoutIndex: number;
  onChoice: (selectedIndex: number) => void;
}

export const MultiviewChooser = (props: MultiviewChooserProps) => {
  return (
    <Select
      id="layout"
      value={props.layoutIndex}
      onChange={(e) => {
        props.onChoice(
          typeof e.target.value === "string"
            ? parseInt(e.target.value)
            : (e.target.value as number),
        );
      }}
      size="small"
      sx={{ color: "inherit", "& svg": { color: "inherit" } }}
    >
      {LAYOUTS.map((e, i) => (
        <MenuItem key={e.className} value={i}>
          {e.name}
        </MenuItem>
      ))}
    </Select>
  );
};

export const MAX_CAMERAS = 9;
export type SelectedCameras = Array<number | null>;
export interface SelectOp {
  selectedIndex: number;
  cameraIndex: number | null;
}

export function selectedReducer(
  old: SelectedCameras,
  op: SelectOp,
): SelectedCameras {
  const selected = Array.isArray(old)
    ? [...old]
    : Array(MAX_CAMERAS).fill(null);
  if (op.cameraIndex !== null) {
    for (let i = 0; i < selected.length; i++) {
      if (selected[i] === op.cameraIndex) {
        selected[i] = null;
      }
    }
  }
  selected[op.selectedIndex] = op.cameraIndex ?? null;
  return selected;
}

export const getInitialSelected = (
  searchParams: URLSearchParams,
  cameras: Camera[],
): SelectedCameras => {
  let result: SelectedCameras | null = null;
  try {
    const fromUrl = searchParams.get("cams");
    if (fromUrl) {
      const parsed = JSON.parse(fromUrl);
      if (Array.isArray(parsed)) {
        result = parsed;
      }
    }
    if (!result) {
      const fromStorage = localStorage.getItem("camsSelected");
      if (fromStorage) {
        const parsed = JSON.parse(fromStorage);
        if (Array.isArray(parsed)) result = parsed;
      }
    }
  } catch (e) {
    console.warn("Failed to parse selected cameras, clearing storage", e);
    localStorage.removeItem("camsSelected");
  }
  // If we have a result from URL or storage, use it
  if (result) return result;
  // Auto-select all available cameras if nothing is selected
  if (cameras.length > 0) {
    const def = Array(MAX_CAMERAS).fill(null);
    // Select all cameras up to MAX_CAMERAS
    const camerasToSelect = Math.min(cameras.length, MAX_CAMERAS);
    for (let i = 0; i < camerasToSelect; i++) {
      def[i] = i;
    }
    return def;
  }
  return Array(MAX_CAMERAS).fill(null);
};

const Multiview = ({
  cameras,
  layoutIndex,
  selected,
  updateSelected,
  renderCamera,
}: MultiviewProps) => {
  const outerRef = React.useRef<HTMLDivElement>(null);

  const currentLayout = LAYOUTS[layoutIndex] || LAYOUTS[0];

  const monoviews = (selected || Array(MAX_CAMERAS).fill(null))
    .slice(0, currentLayout.cameras)
    .map((e, i) => {
      const key = e ?? -1 - i;
      return (
        <Monoview
          key={key}
          cameras={cameras || []}
          cameraIndex={e}
          selectedIndex={i}
          renderCamera={renderCamera}
          updateSelected={updateSelected} // Pass the lifted updateSelected
        />
      );
    });

  return (
    <Box
      ref={outerRef}
      sx={{
        flex: "1 0 0",
        color: "white",
        overflow: "hidden",
        "& > .mid": {
          width: "100%",
          height: "100%",
          position: "relative",
          display: "inline-block",
        },
      }}
    >
      <div className="mid">
        <Box
          className={currentLayout.className}
          sx={{
            position: "absolute",
            width: "100%",
            height: "100%",
            backgroundColor: "#000",
            overflow: "hidden",
            display: "grid",
            gridGap: "1px",
            "&.solo": { gridTemplateColumns: "100%", gridTemplateRows: "100%" },
            "&.dual": {
              gridTemplateColumns: {
                xs: "100%",
                sm: "100%",
                md: "repeat(2, 50%)",
              },
              gridTemplateRows: { xs: "50%", sm: "50%", md: "100%" },
            },
            "&.two-by-two": {
              gridTemplateColumns: "repeat(2, 50%)",
              gridTemplateRows: "repeat(2, 50%)",
            },
            "&.two-by-three": {
              gridTemplateColumns: "repeat(2, 50%)",
              gridTemplateRows: "repeat(3, 33.33%)",
            },
            "&.three-by-two": {
              gridTemplateColumns: "repeat(3, 33.33%)",
              gridTemplateRows: "repeat(2, 50%)",
            },
            "&.main-plus-five, &.three-by-three": {
              gridTemplateColumns: "repeat(3, 33.33%)",
              gridTemplateRows: "repeat(3, 33.33%)",
            },
            "&.main-plus-five > div:nth-of-type(1)": {
              gridColumn: "span 2",
              gridRow: "span 2",
            },
          }}
        >
          {monoviews}
        </Box>
      </div>
    </Box>
  );
};

interface MonoviewProps {
  cameras: Camera[];
  cameraIndex: number | null;
  selectedIndex: number; // The index of this monoview in the selected array
  updateSelected: (op: SelectOp) => void;
  renderCamera: (
    camera: Camera | null,
    chooser: React.JSX.Element,
  ) => React.JSX.Element;
}

const Monoview = (props: MonoviewProps) => {
  const handleChange = (event: SelectChangeEvent<string>) => {
    const {
      target: { value },
    } = event;
    props.updateSelected({
      selectedIndex: props.selectedIndex,
      cameraIndex: value === "null" ? null : parseInt(value),
    });
  };

  const cameras = Array.isArray(props.cameras) ? props.cameras : [];
  const selectedCamera =
    props.cameraIndex !== null && cameras[props.cameraIndex]
      ? cameras[props.cameraIndex]
      : null;

  const chooser = (
    <Select
      value={props.cameraIndex === null ? "null" : props.cameraIndex.toString()}
      onChange={handleChange}
      displayEmpty
      size="small"
      sx={{
        transform: "scale(0.8)",
        backgroundColor: "rgba(50, 50, 50, 0.6)",
        color: "#fff",
        "& svg": { color: "inherit" },
      }}
    >
      <MenuItem value="null">
        <em>(none)</em>
      </MenuItem>
      {cameras.map((e, i) => (
        <MenuItem key={i} value={i}>
          {e.shortName}
        </MenuItem>
      ))}
    </Select>
  );

  const handleDragOver = (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault(); // Allow drop
  };

  const handleDrop = (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    const droppedCameraId = event.dataTransfer.getData("cameraIndex");
    if (droppedCameraId) {
      props.updateSelected({
        selectedIndex: props.selectedIndex,
        cameraIndex: parseInt(droppedCameraId),
      });
    }
  };

  return (
    <Box
      onDragOver={handleDragOver}
      onDrop={handleDrop}
      sx={{
        position: "relative",
        width: "100%",
        height: "100%",
        border:
          props.cameraIndex === null
            ? "2px dashed rgba(255,255,255,0.3)"
            : "none",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      {selectedCamera === null && (
        <span style={{ color: "rgba(255,255,255,0.5)", fontSize: "1.2rem" }}>
          Drop Camera Here
        </span>
      )}
      {props.renderCamera(selectedCamera, chooser)}
    </Box>
  );
};

export default Multiview;
