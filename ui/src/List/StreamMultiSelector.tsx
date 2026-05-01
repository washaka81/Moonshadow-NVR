// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2021 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import Box from "@mui/material/Box";
import { Camera, Stream, StreamType } from "../types";
import Checkbox from "@mui/material/Checkbox";
import { ToplevelResponse } from "../api";
import Paper from "@mui/material/Paper";
import Typography from "@mui/material/Typography";
import Divider from "@mui/material/Divider";

interface Props {
  toplevel: ToplevelResponse;
  selected: Set<number>;
  setSelected: (selected: Set<number>) => void;
}

const StreamMultiSelector = ({ toplevel, selected, setSelected }: Props) => {
  const setStream = (s: Stream, checked: boolean) => {
    const updated = new Set(selected);
    if (checked) {
      updated.add(s.id);
    } else {
      updated.delete(s.id);
    }
    setSelected(updated);
  };

  const toggleType = (st: StreamType) => {
    const updated = new Set(selected);
    let allSelected = true;
    for (const c of toplevel.cameras) {
      const s = c.streams[st];
      if (s !== undefined && !selected.has(s.id)) {
        allSelected = false;
        break;
      }
    }

    for (const c of toplevel.cameras) {
      const s = c.streams[st];
      if (s !== undefined) {
        if (allSelected) updated.delete(s.id);
        else updated.add(s.id);
      }
    }
    setSelected(updated);
  };

  const toggleCamera = (c: Camera) => {
    const updated = new Set(selected);
    let allSelected = true;
    for (const st in c.streams) {
      const s = c.streams[st as StreamType];
      if (s !== undefined && !selected.has(s.id)) {
        allSelected = false;
        break;
      }
    }

    for (const st in c.streams) {
      const s = c.streams[st as StreamType];
      if (s !== undefined) {
        if (allSelected) updated.delete(s.id);
        else updated.add(s.id);
      }
    }
    setSelected(updated);
  };

  return (
    <Paper sx={{ p: 1.5, borderRadius: 2 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', mb: 1, px: 0.5 }}>
        <Typography variant="caption" sx={{ flex: 1, fontWeight: 700, color: 'text.secondary', textTransform: 'uppercase' }}>
          Cameras
        </Typography>
        <Box sx={{ display: 'flex', gap: 2 }}>
          <Typography 
            variant="caption" 
            onClick={() => toggleType("main")}
            sx={{ cursor: 'pointer', fontWeight: 700, color: 'primary.main', '&:hover': { textDecoration: 'underline' } }}
          >
            MAIN
          </Typography>
          <Typography 
            variant="caption" 
            onClick={() => toggleType("sub")}
            sx={{ cursor: 'pointer', fontWeight: 700, color: 'primary.main', '&:hover': { textDecoration: 'underline' } }}
          >
            SUB
          </Typography>
        </Box>
      </Box>
      <Divider sx={{ mb: 1 }} />
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
        {toplevel.cameras.map((c) => (
          <Box key={c.uuid} sx={{ display: 'flex', alignItems: 'center', py: 0.25, '&:hover': { bgcolor: 'rgba(0,0,0,0.02)' }, borderRadius: 1, px: 0.5 }}>
            <Typography 
              variant="body2" 
              onClick={() => toggleCamera(c)}
              sx={{ flex: 1, cursor: 'pointer', fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
            >
              {c.shortName}
            </Typography>
            <Box sx={{ display: 'flex', gap: 1 }}>
              {["main", "sub"].map((st) => {
                const s = c.streams[st as StreamType];
                return (
                  <Checkbox
                    key={st}
                    size="small"
                    disabled={s === undefined}
                    checked={s !== undefined && selected.has(s.id)}
                    color="primary"
                    sx={{ p: 0.5 }}
                    onChange={(e) => s && setStream(s, e.target.checked)}
                  />
                );
              })}
            </Box>
          </Box>
        ))}
      </Box>
    </Paper>
  );
};

export default StreamMultiSelector;
