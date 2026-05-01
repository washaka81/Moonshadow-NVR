// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2021 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import Box from "@mui/material/Box";
import Modal from "@mui/material/Modal";
import Paper from "@mui/material/Paper";
import { useTheme } from "@mui/material/styles";
import Table from "@mui/material/Table";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import Icon from "@mui/material/Icon";
import ErrorIcon from "@mui/icons-material/Error";
import { toZonedTime } from "date-fns-tz";
import { format, addDays } from "date-fns";
import React, { useMemo, useReducer, useState, useEffect } from "react";
import * as api from "../api";
import { Stream } from "../types";
import DisplaySelector, { DEFAULT_DURATION } from "./DisplaySelector";
import StreamMultiSelector from "./StreamMultiSelector";
import TimerangeSelector from "./TimerangeSelector";
import VideoList from "./VideoList";
import { useSearchParams } from "react-router-dom";
import { FrameProps } from "../App";
import IconButton from "@mui/material/IconButton";
import FilterList from "@mui/icons-material/FilterList";
import { FullScreenVideo } from "../components/FullScreenVideo";

interface Props {
  timeZoneName: string;
  toplevel: api.ToplevelResponse;
  Frame: (props: FrameProps) => React.JSX.Element;
}

interface ParsedSearchParams {
  selectedStreamIds: Set<number>;
  split90k: number | undefined;
  trimStartAndEnd: boolean;
  timestampTrack: boolean;
}

interface ParsedSearchParamsAndSetters extends ParsedSearchParams {
  setSelectedStreamIds: (selectedStreamIds: Set<number>) => void;
  setSplit90k: (split90k: number | undefined) => void;
  setTrimStartAndEnd: (trimStartAndEnd: boolean) => void;
  setTimestampTrack: (timestampTrack: boolean) => void;
}

const parseSearchParams = (raw: URLSearchParams, toplevel: api.ToplevelResponse): ParsedSearchParams => {
  const selectedStreamIds = new Set<number>();
  let split90k = DEFAULT_DURATION;
  let trimStartAndEnd = true;
  let timestampTrack = false;
  const sValues = raw.getAll("s");
  for (const v of sValues) {
    selectedStreamIds.add(Number.parseInt(v, 10));
  }
  for (const [key, value] of raw) {
    switch (key) {
      case "split": split90k = value === "inf" ? undefined : Number.parseInt(value, 10); break;
      case "trim": trimStartAndEnd = value === "true"; break;
      case "ts": timestampTrack = value === "true"; break;
    }
  }
  if (sValues.length === 0) {
    for (const s of toplevel.streams.values()) {
      selectedStreamIds.add(s.id);
    }
  }
  return { selectedStreamIds, split90k, trimStartAndEnd, timestampTrack };
};

const useParsedSearchParams = (toplevel: api.ToplevelResponse): ParsedSearchParamsAndSetters => {
  const [search, setSearch] = useSearchParams();
  const { selectedStreamIds, split90k, trimStartAndEnd, timestampTrack } = useMemo(() => parseSearchParams(search, toplevel), [search, toplevel]);

  const setSelectedStreamIds = (newSelectedStreamIds: Set<number>) => {
    const newSearch = new URLSearchParams(search);
    newSearch.delete("s");
    if (newSelectedStreamIds.size > 0) {
      for (const id of newSelectedStreamIds) { newSearch.append("s", id.toString()); }
    }
    setSearch(newSearch);
  };
  const setSplit90k = (newSplit90k: number | undefined) => {
    if (newSplit90k === split90k) return;
    const newSearch = new URLSearchParams(search);
    if (newSplit90k === DEFAULT_DURATION) newSearch.delete("split");
    else if (newSplit90k === undefined) newSearch.set("split", "inf");
    else newSearch.set("split", newSplit90k.toString());
    setSearch(newSearch);
  };
  const setTrimStartAndEnd = (newTrimStartAndEnd: boolean) => {
    if (newTrimStartAndEnd === trimStartAndEnd) return;
    const newSearch = new URLSearchParams(search);
    if (newTrimStartAndEnd === true) newSearch.delete("trim");
    else newSearch.set("trim", "false");
    setSearch(newSearch);
  };
  const setTimestampTrack = (newTimestampTrack: boolean) => {
    if (newTimestampTrack === timestampTrack) return;
    const newSearch = new URLSearchParams(search);
    if (newTimestampTrack === false) newSearch.delete("ts");
    else newSearch.set("ts", "true");
    setSearch(newSearch);
  };
  return { selectedStreamIds, setSelectedStreamIds, split90k, setSplit90k, trimStartAndEnd, setTrimStartAndEnd, timestampTrack, setTimestampTrack };
};

const calcSelectedStreams = (toplevel: api.ToplevelResponse, ids: Set<number>): Set<Stream> => {
  const streams = new Set<Stream>();
  for (const id of ids) {
    const s = toplevel.streams.get(id);
    if (s !== undefined) streams.add(s);
  }
  return streams;
};

const Main = ({ toplevel, timeZoneName, Frame }: Props) => {
  const theme = useTheme();
  const { selectedStreamIds, setSelectedStreamIds, split90k, setSplit90k, trimStartAndEnd, setTrimStartAndEnd, timestampTrack, setTimestampTrack } = useParsedSearchParams(toplevel);
  const [showSelectors, toggleShowSelectors] = useReducer((m: boolean) => !m, true);
  const [range90k, setRange90k] = useState<[number, number] | null>(null);
  const selectedStreams = useMemo(() => calcSelectedStreams(toplevel, selectedStreamIds), [toplevel, selectedStreamIds]);
  const [activeVideo, setActiveVideo] = useState<{ src: string, aspect: [number, number], initialSpeed?: number, filename?: string } | null>(null);

  const formatTime = useMemo(() => {
    return (time90k: number) => format(toZonedTime(new Date(time90k / 90), timeZoneName), "d MMM yyyy HH:mm:ss");
  }, [timeZoneName]);

  useEffect(() => {
    if (range90k === null && selectedStreams.size > 0) {
      // Find the most recent day with recordings
      let maxDayMillis = 0;
      for (const s of selectedStreams) {
        for (const dayStr of Object.keys(s.days)) {
          const dayMillis = new Date(dayStr + "T00:00:00").getTime();
          if (dayMillis > maxDayMillis) {
            maxDayMillis = dayMillis;
          }
        }
      }
      if (maxDayMillis > 0) {
        const startDay90k = maxDayMillis * 90;
        const endDay90k = addDays(new Date(maxDayMillis), 1).getTime() * 90;
        setRange90k([startDay90k, endDay90k]);
      }
    }
  }, [range90k, selectedStreams]);

  const videoLists = [];
  for (const s of selectedStreams) {
    videoLists.push(
      <VideoList
        key={`${s.camera.uuid}-${s.streamType}`}
        stream={s}
        range90k={range90k}
        split90k={split90k}
        trimStartAndEnd={trimStartAndEnd}
        setActiveRecording={(recording) => {
          if (!recording) return;
          const [stream, r] = recording;
          const src = api.recordingUrl(stream.camera.uuid, stream.streamType, r, timestampTrack, trimStartAndEnd ? range90k! : undefined);
          const filename = `recording_${stream.camera.shortName}_${formatTime(r.startTime90k).replace(/[: ]/g, '-')}.mp4`;
          setActiveVideo({ src, aspect: [r.aspectWidth, r.aspectHeight], filename });
        }}
        formatTime={formatTime}
      />,
    );
  }

  const recordingsTable = (
    <TableContainer
      component={Paper}
      sx={{
        mx: 1,
        flex: 1,
        height: "100%",
        overflow: "auto",
        "& .streamHeader": {
          background: theme.vars!.palette.header,
          color: theme.vars!.palette.headerContrastText,
        },
        "& .MuiTableBody-root:not(:last-child):after": {
          content: "''",
          display: "block",
          height: theme.spacing(2),
        },
        "& tbody .recording": { cursor: "pointer" },
        "& .opt": { [theme.breakpoints.down("lg")]: { display: "none" } },
      }}
    >
      <Table size="small" stickyHeader sx={{ minWidth: 650, borderCollapse: 'separate' }}>
        <TableHead>
          <TableRow sx={{ '& th': { fontWeight: 700, color: 'text.secondary', textTransform: 'uppercase', fontSize: '0.75rem', bgcolor: 'background.paper', borderBottom: '1px solid rgba(255,255,255,0.1)', zIndex: 10 } }}>
            <TableCell padding="checkbox" component="th" />
            <TableCell align="left" component="th">Start</TableCell>
            <TableCell align="left" component="th">End<Icon sx={{ verticalAlign: "bottom", marginLeft: ".5em", visibility: 'hidden' }}><ErrorIcon /></Icon></TableCell>
            <TableCell align="right" className="opt" component="th">Resolution</TableCell>
            <TableCell align="right" className="opt" component="th">FPS</TableCell>
            <TableCell align="right" className="opt" component="th">Storage</TableCell>
            <TableCell align="right" component="th">Bitrate / AI</TableCell>
          </TableRow>
        </TableHead>
        {videoLists}
      </Table>
    </TableContainer>
  );

  return (
    <Frame
      activityMenuPart={
        <IconButton onClick={toggleShowSelectors} color="inherit" sx={showSelectors ? { border: `1px solid rgba(255,255,255,0.2)` } : {}} size="small">
          <FilterList />
        </IconButton>
      }
    >
      <Box sx={{ display: "flex", flexWrap: { xs: "wrap", md: "nowrap" }, margin: theme.spacing(2), height: "calc(100% - 32px)", overflow: "hidden" }}>
        <Box sx={{ display: showSelectors ? "flex" : "none", width: { xs: "100%", md: "300px" }, flexShrink: 0, gap: 1, mb: { xs: 2, md: 0 }, flexDirection: "column", overflowY: "auto", height: "100%", pr: 1 }}>
          <StreamMultiSelector toplevel={toplevel} selected={selectedStreamIds} setSelected={setSelectedStreamIds} />
          <TimerangeSelector selectedStreams={selectedStreams} setRange90k={setRange90k} timeZoneName={timeZoneName} />
          <DisplaySelector split90k={split90k} setSplit90k={setSplit90k} trimStartAndEnd={trimStartAndEnd} setTrimStartAndEnd={setTrimStartAndEnd} timestampTrack={timestampTrack} setTimestampTrack={setTimestampTrack} />
        </Box>
        {videoLists.length > 0 && recordingsTable}
        {activeVideo != null && (
          <Modal open onClose={() => setActiveVideo(null)} sx={{ display: "flex", alignItems: "center", justifyContent: "center" }}>
            <FullScreenVideo
              onClose={() => setActiveVideo(null)}
              src={activeVideo.src}
              aspect={activeVideo.aspect}
              initialSpeed={activeVideo.initialSpeed}
              filename={activeVideo.filename}
            />
          </Modal>
        )}
      </Box>
    </Frame>
  );
};

export default Main;
