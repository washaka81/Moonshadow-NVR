// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2025 Moonshadow NVR Contributors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import React, { useEffect, useState, useCallback } from "react";
import Container from "@mui/material/Container";
import Typography from "@mui/material/Typography";
import Box from "@mui/material/Box";
import Paper from "@mui/material/Paper";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import TextField from "@mui/material/TextField";
import IconButton from "@mui/material/IconButton";
import FormControl from "@mui/material/FormControl";
import InputLabel from "@mui/material/InputLabel";
import Select from "@mui/material/Select";
import MenuItem from "@mui/material/MenuItem";
import Chip from "@mui/material/Chip";
import Tooltip from "@mui/material/Tooltip";
import ToggleButton from "@mui/material/ToggleButton";
import ToggleButtonGroup from "@mui/material/ToggleButtonGroup";
import Timeline from "@mui/lab/Timeline";
import TimelineItem, { timelineItemClasses } from "@mui/lab/TimelineItem";
import TimelineSeparator from "@mui/lab/TimelineSeparator";
import TimelineConnector from "@mui/lab/TimelineConnector";
import TimelineContent from "@mui/lab/TimelineContent";
import TimelineDot from "@mui/lab/TimelineDot";
import TimelineOppositeContent from "@mui/lab/TimelineOppositeContent";
import FormatListBulletedIcon from "@mui/icons-material/FormatListBulleted";
import ViewTimelineIcon from "@mui/icons-material/ViewTimeline";
import PersonIcon from "@mui/icons-material/Person";
import DirectionsCarIcon from "@mui/icons-material/DirectionsCar";
import TwoWheelerIcon from "@mui/icons-material/TwoWheeler";
import DirectionsBusIcon from "@mui/icons-material/DirectionsBus";
import LocalShippingIcon from "@mui/icons-material/LocalShipping";
import HelpOutlineIcon from "@mui/icons-material/HelpOutline";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import DownloadIcon from "@mui/icons-material/Download";
import RefreshIcon from "@mui/icons-material/Refresh";
import Modal from "@mui/material/Modal";
import { FullScreenVideo } from "../components/FullScreenVideo";
import { format, toZonedTime } from "date-fns-tz";
import * as api from "../api";
import { AiEvent } from "../types";
import { FrameProps } from "../App";

interface EnhancedAiEvent extends AiEvent {
  formattedDate?: string;
  formattedTime?: string;
}

const getColorForType = (type: string): "primary" | "success" | "warning" | "secondary" | "default" => {
    switch (type.toLowerCase()) {
      case 'person': return 'primary';
      case 'car': return 'success';
      case 'motorcycle': return 'warning';
      case 'license_plate': return 'secondary';
      default: return 'default';
    }
};

const getIconForType = (type: string) => {
    switch (type.toLowerCase()) {
      case 'person': return <PersonIcon fontSize="small" />;
      case 'car': return <DirectionsCarIcon fontSize="small" />;
      case 'motorcycle': return <TwoWheelerIcon fontSize="small" />;
      case 'bus': return <DirectionsBusIcon fontSize="small" />;
      case 'truck': return <LocalShippingIcon fontSize="small" />;
      case 'license_plate': return <DirectionsCarIcon fontSize="small" />;
      default: return <HelpOutlineIcon fontSize="small" />;
    }
};

function renderIdentification(payload: string, type: string) {
  try {
    const data = JSON.parse(payload);
    
    if (type === 'license_plate' || data.plate) {
      return (
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
            <Typography variant="body2" sx={{ 
            fontFamily: 'monospace', 
            fontWeight: 800, 
            color: 'secondary.light',
            bgcolor: 'rgba(156, 39, 176, 0.1)',
            px: 1,
            borderRadius: 0.5,
            display: 'inline-block',
            letterSpacing: 1.5,
            width: 'fit-content'
            }}>
            {data.plate || payload}
            </Typography>
            {data.conf !== undefined && (
                <Typography variant="caption" sx={{ color: 'text.secondary', fontSize: '0.65rem' }}>
                    {Math.round(data.conf * 100)}% CONFIDENCE
                </Typography>
            )}
        </Box>
      );
    }
    
    if (data.conf !== undefined) {
      const conf = Math.round(data.conf * 100);
      return (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <Box sx={{ 
            width: 40, 
            height: 4, 
            bgcolor: 'rgba(255,255,255,0.1)', 
            borderRadius: 2, 
            overflow: 'hidden',
            display: { xs: 'none', sm: 'block' }
          }}>
            <Box sx={{ 
              width: `${conf}%`, 
              height: '100%', 
              bgcolor: conf > 80 ? 'success.main' : conf > 50 ? 'warning.main' : 'error.main' 
            }} />
          </Box>
          <Typography variant="caption" sx={{ fontWeight: 800, color: 'text.secondary' }}>
            {conf}% MATCH
          </Typography>
        </Box>
      );
    }
    
    return <Typography variant="body2">{payload}</Typography>;
  } catch {
    return <Typography variant="body2">{payload}</Typography>;
  }
}

function renderObjectType(event: EnhancedAiEvent) {
    const typeDisplay = event.type_.replace('_', ' ');
    let vehicleType = '';
    
    try {
        const data = JSON.parse(event.value);
        if (data.type) vehicleType = data.type;
    } catch {
        // Ignore JSON parse errors
    }

    return (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <Box sx={{ color: `${getColorForType(event.type_)}.main`, display: 'flex' }}>
                {getIconForType(vehicleType || event.type_)}
            </Box>
            <Box>
                <Typography variant="body2" sx={{ textTransform: 'capitalize', fontWeight: 500 }}>
                    {typeDisplay}
                </Typography>
                {vehicleType && vehicleType !== event.type_ && (
                    <Typography variant="caption" sx={{ textTransform: 'uppercase', color: 'text.secondary', fontSize: '0.6rem', fontWeight: 700 }}>
                        {vehicleType}
                    </Typography>
                )}
            </Box>
        </Box>
    );
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
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<"table" | "timeline">("table");
  const [activeVideo, setActiveVideo] = useState<{ src: string, filename: string } | null>(null);

  const fetchEvents = useCallback(async () => {
    const req: api.AiEventsRequest = {};
    if (typeFilter) req.type = typeFilter;
    if (cameraIdFilter) req.cameraId = parseInt(cameraIdFilter, 10);
    if (limitFilter) req.limit = parseInt(limitFilter, 10);

    setLoading(true);
    setError(null);
    
    try {
      const resp = await api.aiEvents(req, {});
      setLoading(false);
      if (resp.status === "success") {
        const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
        const enhancedEvents = resp.response.events.map(event => {
          const date = new Date(event.time_90k / 90);
          const zonedDate = toZonedTime(date, timeZone);
          return {
            ...event,
            formattedDate: format(zonedDate, "yyyy-MM-dd", { timeZone }),
            formattedTime: format(zonedDate, "HH:mm:ss.SSS", { timeZone }),
          };
        });
        setEvents(enhancedEvents);
      } else {
        setError(`Error: ${resp.status === "error" ? resp.message : resp.status}`);
      }
    } catch (e) {
      setLoading(false);
      setError(`Critical error: ${e}`);
    }
  }, [typeFilter, cameraIdFilter, limitFilter]);

  useEffect(() => {
    fetchEvents();
    const interval = setInterval(fetchEvents, 10000);
    return () => clearInterval(interval);
  }, [fetchEvents]);

  return (
    <Frame>
      <Box sx={{ flex: 1, overflowY: 'auto' }}>
        <Container maxWidth="lg" sx={{ py: 4 }}>
          <Box sx={{ mb: 4, display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 2 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
              <Typography variant="h4" component="h1" sx={{ fontWeight: 700, letterSpacing: -1, color: 'primary.main' }}>
                AI Events
              </Typography>
              <ToggleButtonGroup
                value={viewMode}
                exclusive
                onChange={(e, newMode) => newMode && setViewMode(newMode)}
                size="small"
                sx={{ ml: 2, '& .MuiToggleButton-root': { py: 0.5, px: 1.5 } }}
              >
                <ToggleButton value="table" aria-label="table view">
                  <Tooltip title="Table View"><FormatListBulletedIcon fontSize="small" /></Tooltip>
                </ToggleButton>
                <ToggleButton value="timeline" aria-label="timeline view">
                  <Tooltip title="Timeline View"><ViewTimelineIcon fontSize="small" /></Tooltip>
                </ToggleButton>
              </ToggleButtonGroup>
            </Box>
            <Box sx={{ display: 'flex', gap: 1 }}>
              <Chip 
                label={`${events.length} Detections`} 
                variant="outlined" 
                sx={{ borderRadius: 1.5, fontWeight: 600 }} 
              />
              <IconButton onClick={fetchEvents} disabled={loading} size="small">
                  <RefreshIcon />
              </IconButton>
            </Box>
          </Box>

          <Paper sx={{ p: 2, mb: 3, display: 'flex', gap: 2, flexWrap: 'wrap', alignItems: 'center', borderRadius: 2, bgcolor: 'background.paper', border: '1px solid rgba(255,255,255,0.05)' }}>
            <FormControl size="small" sx={{ minWidth: 150 }}>
              <InputLabel>Type</InputLabel>
              <Select value={typeFilter} label="Type" onChange={(e) => setTypeFilter(e.target.value as string)}>
                <MenuItem value=""><em>All Objects</em></MenuItem>
                <MenuItem value="license_plate">License Plates</MenuItem>
                <MenuItem value="person">Persons</MenuItem>
                <MenuItem value="car">Cars</MenuItem>
              </Select>
            </FormControl>
            <TextField label="Camera ID" size="small" value={cameraIdFilter} onChange={(e) => setCameraIdFilter(e.target.value)} type="number" sx={{ width: 120 }} />
            <TextField label="Limit" size="small" value={limitFilter} onChange={(e) => setLimitFilter(e.target.value)} type="number" sx={{ width: 100 }} />
          </Paper>

          {error && (
             <Paper sx={{ p: 2, mb: 2, bgcolor: 'rgba(244, 67, 54, 0.1)', border: '1px solid rgba(244, 67, 54, 0.3)', borderRadius: 2 }}>
                <Typography color="error" variant="body2">{error}</Typography>
             </Paper>
          )}

          {viewMode === "table" ? (
            <TableContainer component={Paper} sx={{ 
              bgcolor: 'background.paper', 
              borderRadius: 3, 
              boxShadow: '0 10px 40px rgba(0,0,0,0.3)',
              border: '1px solid rgba(255,255,255,0.05)',
              overflow: 'hidden'
            }}>
              <Table sx={{ minWidth: 650 }} size="small">
                <TableHead>
                  <TableRow sx={{ bgcolor: 'rgba(255,255,255,0.02)' }}>
                    <TableCell sx={{ fontWeight: 800, color: 'text.secondary', py: 2 }}>Thumbnail</TableCell>
                    <TableCell sx={{ fontWeight: 800, color: 'text.secondary' }}>Timestamp</TableCell>
                    <TableCell sx={{ fontWeight: 800, color: 'text.secondary' }}>Camera</TableCell>
                    <TableCell sx={{ fontWeight: 800, color: 'text.secondary' }}>Detection</TableCell>
                    <TableCell sx={{ fontWeight: 800, color: 'text.secondary' }}>Identification</TableCell>
                    <TableCell align="center" sx={{ fontWeight: 800, color: 'text.secondary' }}>Actions</TableCell>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {events.length > 0 ? (
                    events.map((event, idx) => (
                      <TableRow key={idx} hover sx={{ '&:last-child td, &:last-child th': { border: 0 }, '&:hover': { bgcolor: 'rgba(255,255,255,0.03)' } }}>
                        <TableCell>
                          {event.video_link ? (
                            <Box sx={{ width: 120, height: 68, borderRadius: 2, overflow: 'hidden', bgcolor: '#000', position: 'relative' }}>
                              <video 
                                src={`${event.video_link}#t=0.1`} 
                                muted 
                                playsInline 
                                preload="metadata"
                                style={{ width: '100%', height: '100%', objectFit: 'cover' }}
                                onMouseOver={(e) => (e.target as HTMLVideoElement).play().catch(() => {})}
                                onMouseOut={(e) => {
                                  const v = e.target as HTMLVideoElement;
                                  v.pause();
                                  v.currentTime = 0.1;
                                }}
                              />
                            </Box>
                          ) : (
                            <Box sx={{ width: 120, height: 68, bgcolor: 'rgba(255,255,255,0.05)', borderRadius: 2, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                              <Typography variant="caption" color="text.secondary">No video</Typography>
                            </Box>
                          )}
                        </TableCell>
                        <TableCell>
                          <Typography variant="body2" sx={{ fontWeight: 600 }}>{event.formattedDate}</Typography>
                          <Typography variant="caption" sx={{ color: 'text.secondary', fontFamily: 'monospace' }}>{event.formattedTime}</Typography>
                        </TableCell>
                        <TableCell>
                          <Chip 
                            size="small" 
                            label={`CAM #${event.camera_id}`} 
                            sx={{ bgcolor: 'rgba(255,255,255,0.05)', fontWeight: 700, borderRadius: 1 }} 
                          />
                        </TableCell>
                        <TableCell>
                          {renderObjectType(event)}
                        </TableCell>
                        <TableCell>
                          {renderIdentification(event.value, event.type_)}
                        </TableCell>
                        <TableCell align="center">
                          <Box sx={{ display: 'flex', gap: 1, justifyContent: 'center' }}>
                            <Tooltip title="View Recording">
                              <IconButton 
                                size="small" 
                                onClick={() => {
                                  if (event.video_link) {
                                    setActiveVideo({
                                      src: event.video_link,
                                      filename: `detection_${event.type_}_${event.formattedDate}_${event.formattedTime?.replace(/[:.]/g, '-')}.mp4`
                                    });
                                  }
                                }}
                                sx={{ 
                                  color: '#fff', 
                                  bgcolor: 'rgba(255,255,255,0.05)', 
                                  '&:hover': { bgcolor: 'primary.main', transform: 'scale(1.1)' },
                                  transition: '0.2s'
                                }}
                              >
                                <PlayArrowIcon fontSize="small" />
                              </IconButton>
                            </Tooltip>                            <Tooltip title="Download MP4">
                              <IconButton 
                                size="small" 
                                component="a"
                                href={event.video_link || "#"}
                                download={`detection_${event.type_}_${event.formattedDate}_${event.formattedTime?.replace(/[:.]/g, '-')}.mp4`}
                                sx={{ 
                                  color: 'text.secondary', 
                                  '&:hover': { color: '#fff' }
                                }}
                              >
                                <DownloadIcon fontSize="small" />
                              </IconButton>
                            </Tooltip>
                          </Box>
                        </TableCell>
                      </TableRow>
                    ))
                  ) : (
                    <TableRow>
                      <TableCell colSpan={6} align="center" sx={{ py: 10 }}>
                        <Typography variant="body1" sx={{ color: 'text.secondary' }}>
                          {loading ? "Analyzing database..." : "No detections found."}
                        </Typography>
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>
            </TableContainer>
          ) : (
            <Box sx={{ bgcolor: 'background.paper', borderRadius: 3, p: 2, boxShadow: '0 10px 40px rgba(0,0,0,0.3)', border: '1px solid rgba(255,255,255,0.05)' }}>
              {events.length > 0 ? (
                <Timeline sx={{
                  [`& .${timelineItemClasses.root}:before`]: {
                    flex: 0,
                    padding: 0,
                  },
                }}>
                  {events.map((event, idx) => (
                    <TimelineItem key={idx}>
                      <TimelineOppositeContent sx={{ flex: '0.1', textAlign: 'right', pt: 2 }}>
                        <Typography variant="body2" sx={{ fontWeight: 600 }}>{event.formattedDate}</Typography>
                        <Typography variant="caption" sx={{ color: 'text.secondary', fontFamily: 'monospace' }}>{event.formattedTime}</Typography>
                      </TimelineOppositeContent>
                      <TimelineSeparator>
                        <TimelineDot sx={{ bgcolor: `${getColorForType(event.type_)}.main`, mt: 2 }}>
                          {getIconForType(event.type_)}
                        </TimelineDot>
                        {idx < events.length - 1 && <TimelineConnector />}
                      </TimelineSeparator>
                      <TimelineContent sx={{ py: '12px', px: 2 }}>
                        <Paper sx={{ p: 2, bgcolor: 'rgba(255,255,255,0.02)', border: '1px solid rgba(255,255,255,0.05)', display: 'flex', gap: 3, alignItems: 'center', flexWrap: 'wrap' }}>
                          <Box sx={{ flexShrink: 0 }}>
                            {event.video_link ? (
                              <Box sx={{ width: 160, height: 90, borderRadius: 2, overflow: 'hidden', bgcolor: '#000', position: 'relative' }}>
                                <video 
                                  src={`${event.video_link}#t=0.1`} 
                                  muted 
                                  playsInline 
                                  preload="metadata"
                                  style={{ width: '100%', height: '100%', objectFit: 'cover' }}
                                  onMouseOver={(e) => (e.target as HTMLVideoElement).play().catch(() => {})}
                                  onMouseOut={(e) => {
                                    const v = e.target as HTMLVideoElement;
                                    v.pause();
                                    v.currentTime = 0.1;
                                  }}
                                />
                              </Box>
                            ) : (
                              <Box sx={{ width: 160, height: 90, bgcolor: 'rgba(255,255,255,0.05)', borderRadius: 2, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                                <Typography variant="caption" color="text.secondary">No video</Typography>
                              </Box>
                            )}
                          </Box>
                          
                          <Box sx={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 1 }}>
                            <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
                              <Chip 
                                size="small" 
                                label={`CAM #${event.camera_id}`} 
                                sx={{ bgcolor: 'rgba(255,255,255,0.05)', fontWeight: 700, borderRadius: 1 }} 
                              />
                              {renderObjectType(event)}
                            </Box>
                            <Box>
                              {renderIdentification(event.value, event.type_)}
                            </Box>
                          </Box>

                          <Box sx={{ display: 'flex', gap: 1 }}>
                            <Tooltip title="View Recording">
                              <IconButton 
                                onClick={() => {
                                  if (event.video_link) {
                                    setActiveVideo({
                                      src: event.video_link,
                                      filename: `detection_${event.type_}_${event.formattedDate}_${event.formattedTime?.replace(/[:.]/g, '-')}.mp4`
                                    });
                                  }
                                }}
                                sx={{ 
                                  color: '#fff', 
                                  bgcolor: 'rgba(255,255,255,0.05)', 
                                  '&:hover': { bgcolor: 'primary.main', transform: 'scale(1.1)' },
                                  transition: '0.2s'
                                }}
                              >
                                <PlayArrowIcon />
                              </IconButton>
                            </Tooltip>
                            <Tooltip title="Download MP4">
                              <IconButton 
                                component="a"
                                href={event.video_link || "#"}
                                download={`detection_${event.type_}_${event.formattedDate}_${event.formattedTime?.replace(/[:.]/g, '-')}.mp4`}
                                sx={{ 
                                  color: 'text.secondary', 
                                  bgcolor: 'rgba(255,255,255,0.02)',
                                  '&:hover': { color: '#fff', bgcolor: 'rgba(255,255,255,0.1)' }
                                }}
                              >
                                <DownloadIcon />
                              </IconButton>
                            </Tooltip>
                          </Box>
                        </Paper>
                      </TimelineContent>
                    </TimelineItem>
                  ))}
                </Timeline>
              ) : (
                <Box sx={{ py: 10, textAlign: 'center' }}>
                  <Typography variant="body1" sx={{ color: 'text.secondary' }}>
                    {loading ? "Analyzing database..." : "No detections found."}
                  </Typography>
                </Box>
              )}
            </Box>
          )}
        </Container>
      </Box>

      {activeVideo != null && (
        <Modal open onClose={() => setActiveVideo(null)} sx={{ display: "flex", alignItems: "center", justifyContent: "center" }}>
          <FullScreenVideo
            onClose={() => setActiveVideo(null)}
            src={activeVideo.src}
            filename={activeVideo.filename}
          />
        </Modal>
      )}
    </Frame>
  );
}
