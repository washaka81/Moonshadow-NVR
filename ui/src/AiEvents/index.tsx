// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2025 Moonshadow NVR Contributors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import Container from "@mui/material/Container";
import Typography from "@mui/material/Typography";
import React, { useEffect, useState, useCallback } from "react";
import * as api from "../api";
import { AiEvent } from "../types";
import { FrameProps } from "../App";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import Paper from "@mui/material/Paper";
import TextField from "@mui/material/TextField";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import FormControl from "@mui/material/FormControl";
import InputLabel from "@mui/material/InputLabel";
import Select from "@mui/material/Select";
import MenuItem from "@mui/material/MenuItem";
import Chip from "@mui/material/Chip";
import Tooltip from "@mui/material/Tooltip";
import PersonIcon from "@mui/icons-material/Person";
import DirectionsCarIcon from "@mui/icons-material/DirectionsCar";
import { format, toZonedTime } from "date-fns-tz";

interface EnhancedAiEvent extends AiEvent {
  formattedTime?: string;
}

function formatTime90k(time90k: number, timeZoneName: string): string {
  const date = new Date(time90k / 90);
  return format(toZonedTime(date, timeZoneName), "yyyy-MM-dd HH:mm:ss");
}

export default function AiEventsActivity({
  Frame,
}: {
  Frame: React.FC<FrameProps>;
}) {
  const [events, setEvents] = useState<EnhancedAiEvent[]>([]);
  const [typeFilter, setTypeFilter] = useState<string>("");
  const [cameraIdFilter, setCameraIdFilter] = useState<string>("");
  const [limitFilter, setLimitFilter] = useState<string>("100");
  const [loading, setLoading] = useState(false);

  const fetchEvents = useCallback(async () => {
    const req: api.AiEventsRequest = {};
    if (typeFilter) req.type = typeFilter;
    if (cameraIdFilter) req.cameraId = parseInt(cameraIdFilter, 10);
    if (limitFilter) req.limit = parseInt(limitFilter, 10);

    setLoading(true);
    const resp = await api.aiEvents(req, {});
    setLoading(false);
    if (resp.status === "success") {
      setEvents(resp.response.events);
    }
  }, [typeFilter, cameraIdFilter, limitFilter]);

  useEffect(() => {
    fetchEvents();
  }, [fetchEvents]);

  const renderTypeChip = (type: string, value: string) => {
    switch (type) {
      case "plate":
        return (
          <Chip
            size="small"
            icon={<DirectionsCarIcon fontSize="small" />}
            label={value || "Unknown"}
            sx={{ backgroundColor: "#e3f2fd", color: "#1565c0" }}
          />
        );
      case "person_reid":
        return (
          <Chip
            size="small"
            icon={<PersonIcon fontSize="small" />}
            label={value.replace("person_", "P")}
            sx={{ backgroundColor: "#fce4ec", color: "#c2185b" }}
          />
        );
      default:
        return <Chip size="small" label={`${type}: ${value}`} />;
    }
  };

  return (
    <Frame>
      <Container sx={{ mt: 4 }}>
        <Typography variant="h4" gutterBottom>
          AI Events
        </Typography>

        <Box sx={{ display: 'flex', gap: 2, mb: 3, alignItems: 'center', flexWrap: 'wrap' }}>
          <FormControl size="small" sx={{ minWidth: 150 }}>
            <InputLabel>Type</InputLabel>
            <Select
              value={typeFilter}
              label="Type"
              onChange={(e) => setTypeFilter(e.target.value)}
            >
              <MenuItem value=""><em>All</em></MenuItem>
              <MenuItem value="plate">
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                  <DirectionsCarIcon fontSize="small" /> Plate
                </Box>
              </MenuItem>
              <MenuItem value="person_reid">
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                  <PersonIcon fontSize="small" /> Person ReID
                </Box>
              </MenuItem>
              <MenuItem value="object">Object</MenuItem>
            </Select>
          </FormControl>
          <TextField
            label="Camera ID"
            variant="outlined"
            size="small"
            value={cameraIdFilter}
            onChange={(e) => setCameraIdFilter(e.target.value)}
            type="number"
            sx={{ width: 120 }}
          />
          <TextField
            label="Limit"
            variant="outlined"
            size="small"
            value={limitFilter}
            onChange={(e) => setLimitFilter(e.target.value)}
            type="number"
            sx={{ width: 100 }}
          />
          <Button variant="contained" onClick={fetchEvents} disabled={loading}>
            {loading ? "Loading..." : "Refresh"}
          </Button>
        </Box>

        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          Found {events.length} events
        </Typography>

        <TableContainer component={Paper}>
          <Table>
            <TableHead>
              <TableRow>
                <TableCell>Timestamp</TableCell>
                <TableCell>Camera ID</TableCell>
                <TableCell>Detection</TableCell>
                <TableCell>Value</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {events.map((event, idx) => (
                <TableRow key={idx} hover>
                  <TableCell>
                    <Tooltip title={`Raw: ${event.time_90k}`}>
                      <span>{formatTime90k(event.time_90k, Intl.DateTimeFormat().resolvedOptions().timeZone)}</span>
                    </Tooltip>
                  </TableCell>
                  <TableCell>{event.camera_id}</TableCell>
                  <TableCell>{renderTypeChip(event.type_, event.value)}</TableCell>
                  <TableCell>
                    {event.type_ === "plate" ? (
                      <Typography variant="body2" fontFamily="monospace" fontSize="1.1em">
                        {event.value}
                      </Typography>
                    ) : (
                      event.value
                    )}
                  </TableCell>
                </TableRow>
              ))}
              {events.length === 0 && !loading && (
                <TableRow>
                  <TableCell colSpan={4} align="center">
                    No events found
                  </TableCell>
                </TableRow>
              )}
              {loading && (
                <TableRow>
                  <TableCell colSpan={4} align="center">
                    Loading...
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Container>
    </Frame>
  );
}
