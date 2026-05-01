// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Copyright (C) 2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import React, { useEffect, useState, useCallback } from "react";
import Container from "@mui/material/Container";
import Typography from "@mui/material/Typography";
import Box from "@mui/material/Box";
import Paper from "@mui/material/Paper";
import LinearProgress from "@mui/material/LinearProgress";
import Chip from "@mui/material/Chip";
import { FrameProps } from "../App";
import * as api from "../api";
import MemoryIcon from "@mui/icons-material/Memory";
import StorageIcon from "@mui/icons-material/Storage";
import SpeedIcon from "@mui/icons-material/Speed";
import ThermostatIcon from "@mui/icons-material/Thermostat";
import DeveloperBoardIcon from "@mui/icons-material/DeveloperBoard";
import DnsIcon from "@mui/icons-material/Dns";
import TerminalIcon from "@mui/icons-material/Terminal";
import HubIcon from "@mui/icons-material/Hub";

interface SystemInfo {
  fastfetch: {
    host: string;
    os: string;
    kernel: string;
    uptime: string;
    shell: string;
    packages: string;
    cpu_model: string;
  };
  htop: {
    cpu_total: number;
    cpu_cores: number[];
    mem_used: number;
    mem_total: number;
    mem_percent: number;
    swap_used: number;
    swap_total: number;
  };
  nvtop: {
    gpu_usage: number;
    gpu_status: string;
    vram_used: number;
    vram_total: number;
    temp: number;
  };
  disk: {
    free: number;
    total: number;
    percent: number;
  };
  accelerator: string;
}

const defaultInfo: SystemInfo = {
  fastfetch: {
    host: "...",
    os: "...",
    kernel: "...",
    uptime: "...",
    shell: "...",
    packages: "...",
    cpu_model: "...",
  },
  htop: {
    cpu_total: 0,
    cpu_cores: [],
    mem_used: 0,
    mem_total: 0,
    mem_percent: 0,
    swap_used: 0,
    swap_total: 0,
  },
  nvtop: {
    gpu_usage: 0,
    gpu_status: "Idle",
    vram_used: 0,
    vram_total: 0,
    temp: 0,
  },
  disk: { free: 0, total: 0, percent: 0 },
  accelerator: "Intel OpenVINO + Vulkan Compute",
};

const percentColor = (p: number) =>
  p > 90 ? "#f44336" : p > 70 ? "#ff9800" : p > 50 ? "#ffc107" : "#4caf50";

const tempColor = (t: number) =>
  t > 80 ? "#f44336" : t > 65 ? "#ff9800" : t > 50 ? "#ffc107" : "#4caf50";

const formatMBtoGB = (mb: number) => (mb / 1024).toFixed(1);

const ProgressBar: React.FC<{
  label: string;
  value: number;
  max: number;
  unit?: string;
  color?: string;
  showPercent?: boolean;
  compact?: boolean;
}> = ({ label, value, max, unit = "%", color, showPercent = true, compact }) => {
  const p = max > 0 ? (value / max) * 100 : 0;
  const c = color || percentColor(p);
  return (
    <Box sx={{ mb: compact ? 0.8 : 1.5 }}>
      <Box
        sx={{
          display: "flex",
          justifyContent: "space-between",
          mb: 0.4,
          alignItems: "baseline",
        }}
      >
        <Typography
          variant="caption"
          sx={{
            color: "#888",
            fontWeight: 700,
            fontFamily: "monospace",
            fontSize: "0.65rem",
            textTransform: "uppercase",
            letterSpacing: 0.5,
          }}
        >
          {label}
        </Typography>
        <Typography
          variant="caption"
          sx={{
            color: "#ddd",
            fontFamily: "monospace",
            fontSize: "0.7rem",
            fontWeight: 600,
          }}
        >
          {compact
            ? `${Math.round(p)}${unit}`
            : `${Math.round(p)}${unit} (${value} / ${max})`}
        </Typography>
      </Box>
      <LinearProgress
        variant="determinate"
        value={Math.min(p, 100)}
        sx={{
          height: compact ? 5 : 8,
          borderRadius: 0,
          bgcolor: "rgba(255,255,255,0.04)",
          "& .MuiLinearProgress-bar": { bgcolor: c, transition: "transform 0.4s ease" },
        }}
      />
    </Box>
  );
};

const CpuCoreBar: React.FC<{ index: number; usage: number }> = ({
  index,
  usage,
}) => {
  const p = Math.round(usage);
  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1, mb: 0.4 }}>
      <Typography
        sx={{
          fontFamily: "monospace",
          fontSize: "0.65rem",
          width: 28,
          color: "#555",
          textAlign: "right",
        }}
      >
        CPU{index}
      </Typography>
      <Box sx={{ flex: 1, height: 6, bgcolor: "rgba(255,255,255,0.03)" }}>
        <Box
          sx={{
            width: `${p}%`,
            height: "100%",
            bgcolor: percentColor(p),
            transition: "width 0.4s ease",
          }}
        />
      </Box>
      <Typography
        sx={{
          fontFamily: "monospace",
          fontSize: "0.65rem",
          width: 32,
          textAlign: "right",
          color: percentColor(p),
          fontWeight: 700,
        }}
      >
        {p}%
      </Typography>
    </Box>
  );
};

const StatCard: React.FC<{
  icon: React.ReactNode;
  label: string;
  value: string;
  sub?: string;
  color?: string;
}> = ({ icon, label, value, sub, color }) => (
  <Box
    sx={{
      display: "flex",
      alignItems: "center",
      gap: 1.5,
      p: 1.5,
      borderRadius: 1,
      bgcolor: "rgba(255,255,255,0.02)",
      border: "1px solid rgba(255,255,255,0.05)",
    }}
  >
    <Box sx={{ color: color || "#888", opacity: 0.8 }}>{icon}</Box>
    <Box>
      <Typography
        variant="caption"
        sx={{
          color: "#666",
          fontWeight: 700,
          fontSize: "0.6rem",
          textTransform: "uppercase",
          letterSpacing: 0.5,
          display: "block",
        }}
      >
        {label}
      </Typography>
      <Typography
        sx={{
          color: "#eee",
          fontFamily: "monospace",
          fontSize: "0.85rem",
          fontWeight: 600,
        }}
      >
        {value}
      </Typography>
      {sub && (
        <Typography sx={{ color: "#555", fontSize: "0.65rem" }}>{sub}</Typography>
      )}
    </Box>
  </Box>
);

const SectionHeader: React.FC<{
  icon: React.ReactNode;
  title: string;
  color: string;
  badge?: string;
}> = ({ icon, title, color, badge }) => (
  <Box
    sx={{
      display: "flex",
      alignItems: "center",
      gap: 1,
      mb: 0.5,
      pb: 1,
      borderBottom: `1px solid rgba(255,255,255,0.06)`,
    }}
  >
    <Box sx={{ color }}>{icon}</Box>
    <Typography
      variant="subtitle2"
      sx={{
        fontWeight: 800,
        letterSpacing: 1.5,
        color,
        fontFamily: "monospace",
        fontSize: "0.7rem",
        flex: 1,
      }}
    >
      {title}
    </Typography>
    {badge && (
      <Chip
        label={badge}
        size="small"
        sx={{
          height: 18,
          fontSize: "0.6rem",
          fontWeight: 800,
          fontFamily: "monospace",
          bgcolor: `${color}20`,
          color,
          "& .MuiChip-label": { px: 1 },
        }}
      />
    )}
  </Box>
);

export default function AdminDashboard({
  Frame,
  toplevel,
}: {
  Frame: React.FC<FrameProps>;
  toplevel: any;
}) {
  const [info, setInfo] = useState<SystemInfo>(
    toplevel?.systemInfo || defaultInfo
  );
  const [firstLoad, setFirstLoad] = useState(true);

  const fetchInfo = useCallback(async () => {
    const resp = await api.sysinfo({});
    if (resp.status === "success") {
      setInfo(resp.response);
      setFirstLoad(false);
    }
  }, []);

  useEffect(() => {
    if (toplevel?.systemInfo && firstLoad) {
      setInfo(toplevel.systemInfo);
      setFirstLoad(false);
    }
  }, [toplevel, firstLoad]);

  useEffect(() => {
    fetchInfo();
    const timer = setInterval(fetchInfo, 2000);
    return () => clearInterval(timer);
  }, [fetchInfo]);

  const memTotalGB = parseFloat(formatMBtoGB(info.htop.mem_total));
  const memUsedGB = parseFloat(formatMBtoGB(info.htop.mem_used));
  const swapTotalGB = parseFloat(formatMBtoGB(info.htop.swap_total));
  const swapUsedGB = parseFloat(formatMBtoGB(info.htop.swap_used));
  const gpuVRAMGB = info.nvtop.vram_total > 0
    ? info.nvtop.vram_total
    : 0;

  return (
    <Frame>
      <Container
        maxWidth="xl"
        sx={{ mt: 3, mb: 6, overflowY: "auto", height: "calc(100vh - 48px)" }}
      >
        <Box
          sx={{
            display: "flex",
            alignItems: "center",
            gap: 1.5,
            mb: 3,
            pb: 1.5,
            borderBottom: "1px solid rgba(255,255,255,0.06)",
          }}
        >
          <HubIcon sx={{ color: "#64b5f6", fontSize: 28 }} />
          <Box>
            <Typography
              sx={{
                fontFamily: "monospace",
                fontWeight: 800,
                fontSize: "1.1rem",
                color: "#eee",
                letterSpacing: 1,
              }}
            >
              SYSTEM MONITOR
            </Typography>
            <Typography variant="caption" sx={{ color: "#666", fontFamily: "monospace", fontSize: "0.6rem" }}>
              MOONSHADOW NVR — {info.fastfetch.host}
            </Typography>
          </Box>
        </Box>

        {/* Top stat chips row */}
        <Box
          sx={{
            display: "grid",
            gridTemplateColumns: {
              xs: "1fr",
              sm: "1fr 1fr",
              md: "repeat(4, 1fr)",
            },
            gap: 1.5,
            mb: 2,
          }}
        >
          <StatCard
            icon={<DeveloperBoardIcon />}
            label="CPU"
            value={`${info.htop.cpu_total.toFixed(1)}%`}
            sub={`${info.htop.cpu_cores.length} cores @ ${info.fastfetch.cpu_model}`}
            color="#2196f3"
          />
          <StatCard
            icon={<MemoryIcon />}
            label="Memory"
            value={`${info.htop.mem_percent.toFixed(1)}%`}
            sub={`${memUsedGB.toFixed(1)} / ${memTotalGB.toFixed(1)} GB`}
            color="#26d07c"
          />
          <StatCard
            icon={<ThermostatIcon />}
            label="Temperature"
            value={`${info.nvtop.temp.toFixed(0)}°C`}
            sub={info.nvtop.temp > 70 ? "HIGH" : info.nvtop.temp > 50 ? "WARM" : "NORMAL"}
            color={tempColor(info.nvtop.temp)}
          />
          <StatCard
            icon={<StorageIcon />}
            label="Disk"
            value={`${info.disk.percent.toFixed(1)}%`}
            sub={`${info.disk.free} / ${info.disk.total} GB free`}
            color="#ff9800"
          />
        </Box>

        <Box
          sx={{
            display: "grid",
            gridTemplateColumns: { xs: "1fr", md: "1fr 1fr" },
            gap: 2,
          }}
        >
          {/* LEFT COLUMN: CPU + Memory */}
          <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
            {/* CPU Section */}
            <Paper
              sx={{
                p: 2.5,
                bgcolor: "rgba(0,0,0,0.3)",
                border: "1px solid rgba(255,255,255,0.06)",
              }}
            >
              <SectionHeader
                icon={<SpeedIcon fontSize="small" />}
                title="CPU"
                color="#2196f3"
                badge={`${info.htop.cpu_cores.length} CORES`}
              />
              <Box sx={{ mt: 0.5 }}>
                <Box
                  sx={{
                    display: "grid",
                    gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" },
                    gap: 0.5,
                    mb: 0.5,
                  }}
                >
                  {(info.htop.cpu_cores || []).map(
                    (usage: number, i: number) => (
                      <CpuCoreBar key={i} index={i} usage={usage} />
                    )
                  )}
                </Box>
                <Box sx={{ mt: 2 }}>
                  <ProgressBar
                    label="CPU Total"
                    value={info.htop.cpu_total}
                    max={100}
                    compact
                  />
                </Box>
              </Box>
            </Paper>

            {/* Memory Section */}
            <Paper
              sx={{
                p: 2.5,
                bgcolor: "rgba(0,0,0,0.3)",
                border: "1px solid rgba(255,255,255,0.06)",
              }}
            >
              <SectionHeader
                icon={<MemoryIcon fontSize="small" />}
                title="MEMORY"
                color="#26d07c"
              />
              <Box sx={{ mt: 1 }}>
                <ProgressBar
                  label="RAM"
                  value={info.htop.mem_used}
                  max={info.htop.mem_total}
                  unit="%"
                  color="#26d07c"
                />
                <ProgressBar
                  label="SWAP"
                  value={info.htop.swap_used}
                  max={info.htop.swap_total || 1}
                  unit="%"
                  color="#ff9800"
                />
                <Box
                  sx={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr",
                    gap: 1,
                    mt: 2,
                  }}
                >
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      RAM USED
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: "#26d07c", fontSize: "0.9rem", fontWeight: 700 }}>
                      {memUsedGB.toFixed(2)} GB
                    </Typography>
                  </Box>
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      RAM TOTAL
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: "#26d07c", fontSize: "0.9rem", fontWeight: 700 }}>
                      {memTotalGB.toFixed(2)} GB
                    </Typography>
                  </Box>
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      SWAP USED
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: "#ff9800", fontSize: "0.9rem", fontWeight: 700 }}>
                      {swapUsedGB.toFixed(2)} GB
                    </Typography>
                  </Box>
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      SWAP TOTAL
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: "#ff9800", fontSize: "0.9rem", fontWeight: 700 }}>
                      {swapTotalGB.toFixed(2)} GB
                    </Typography>
                  </Box>
                </Box>
              </Box>
            </Paper>
          </Box>

          {/* RIGHT COLUMN: GPU + Disk + System info */}
          <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
            {/* GPU / iGPU Section */}
            <Paper
              sx={{
                p: 2.5,
                bgcolor: "rgba(0,0,0,0.3)",
                border: "1px solid rgba(255,255,255,0.06)",
              }}
            >
              <SectionHeader
                icon={<DeveloperBoardIcon fontSize="small" />}
                title="GPU / iGPU"
                color="#ce93d8"
                badge={info.nvtop.gpu_status.toUpperCase()}
              />
              <Box sx={{ mt: 1 }}>
                <ProgressBar
                  label="GPU Usage"
                  value={info.nvtop.gpu_usage}
                  max={100}
                  unit="%"
                  color="#ce93d8"
                />
                {gpuVRAMGB > 0 && (
                  <ProgressBar
                    label="VRAM"
                    value={info.nvtop.vram_used}
                    max={info.nvtop.vram_total}
                    unit="%"
                    color="#7b1fa2"
                  />
                )}
                <Box
                  sx={{
                    display: "grid",
                    gridTemplateColumns: gpuVRAMGB > 0 ? "1fr 1fr 1fr" : "1fr 1fr",
                    gap: 1,
                    mt: 2,
                  }}
                >
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      GPU LOAD
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: "#ce93d8", fontSize: "0.9rem", fontWeight: 700 }}>
                      {info.nvtop.gpu_usage.toFixed(1)}%
                    </Typography>
                  </Box>
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      TEMP
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: tempColor(info.nvtop.temp), fontSize: "0.9rem", fontWeight: 700 }}>
                      {info.nvtop.temp.toFixed(1)}°C
                    </Typography>
                  </Box>
                  {gpuVRAMGB > 0 && (
                    <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                      <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                        VRAM
                      </Typography>
                      <Typography sx={{ fontFamily: "monospace", color: "#7b1fa2", fontSize: "0.9rem", fontWeight: 700 }}>
                        {info.nvtop.vram_used}M
                      </Typography>
                    </Box>
                  )}
                </Box>
              </Box>
            </Paper>

            {/* Disk Section */}
            <Paper
              sx={{
                p: 2.5,
                bgcolor: "rgba(0,0,0,0.3)",
                border: "1px solid rgba(255,255,255,0.06)",
              }}
            >
              <SectionHeader
                icon={<StorageIcon fontSize="small" />}
                title="STORAGE"
                color="#ff9800"
              />
              <Box sx={{ mt: 1 }}>
                <ProgressBar
                  label="Disk Usage"
                  value={info.disk.percent}
                  max={100}
                  unit="%"
                  color="#ff9800"
                />
                <Box
                  sx={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr",
                    gap: 1,
                    mt: 2,
                  }}
                >
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      FREE
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: "#4caf50", fontSize: "0.9rem", fontWeight: 700 }}>
                      {info.disk.free} GB
                    </Typography>
                  </Box>
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      TOTAL
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: "#ff9800", fontSize: "0.9rem", fontWeight: 700 }}>
                      {info.disk.total} GB
                    </Typography>
                  </Box>
                </Box>
              </Box>
            </Paper>

            {/* System Info Section */}
            <Paper
              sx={{
                p: 2.5,
                bgcolor: "rgba(0,0,0,0.3)",
                border: "1px solid rgba(255,255,255,0.06)",
              }}
            >
              <SectionHeader
                icon={<DnsIcon fontSize="small" />}
                title="SYSTEM"
                color="#64b5f6"
              />
              <Box sx={{ mt: 1 }}>
                <Box
                  sx={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr",
                    gap: 1,
                  }}
                >
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      HOSTNAME
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: "#64b5f6", fontSize: "0.9rem", fontWeight: 700 }}>
                      {info.fastfetch.host}
                    </Typography>
                  </Box>
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      OS
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: "#64b5f6", fontSize: "0.9rem", fontWeight: 700 }}>
                      {info.fastfetch.os}
                    </Typography>
                  </Box>
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      KERNEL
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: "#64b5f6", fontSize: "0.75rem", fontWeight: 600 }}>
                      {info.fastfetch.kernel}
                    </Typography>
                  </Box>
                  <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                    <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                      UPTIME
                    </Typography>
                    <Typography sx={{ fontFamily: "monospace", color: "#64b5f6", fontSize: "0.9rem", fontWeight: 700 }}>
                      {info.fastfetch.uptime}
                    </Typography>
                  </Box>
                </Box>
              </Box>
            </Paper>
          </Box>
        </Box>

        {/* Bottom row: Accelerator + Logs */}
        <Box
          sx={{
            display: "grid",
            gridTemplateColumns: { xs: "1fr", md: "1fr 1fr" },
            gap: 2,
            mt: 2,
          }}
        >
          {/* Accelerator Info */}
          <Paper
            sx={{
              p: 2.5,
              bgcolor: "rgba(0,0,0,0.3)",
              border: "1px solid rgba(255,255,255,0.06)",
            }}
          >
            <SectionHeader
              icon={<HubIcon fontSize="small" />}
              title="ACCELERATOR"
              color="#4db6ac"
              badge="AI INFERENCE"
            />
            <Box sx={{ mt: 1 }}>
              <Box
                sx={{
                  p: 2,
                  bgcolor: "rgba(77,182,172,0.08)",
                  border: "1px solid rgba(77,182,172,0.2)",
                  borderRadius: 1,
                }}
              >
                <Typography
                  sx={{
                    fontFamily: "monospace",
                    fontSize: "0.9rem",
                    color: "#4db6ac",
                    fontWeight: 700,
                    mb: 0.5,
                  }}
                >
                  {info.accelerator}
                </Typography>
                <Typography variant="caption" sx={{ color: "#555", fontFamily: "monospace", fontSize: "0.65rem", display: "block" }}>
                  AI COMPUTE ENGINE
                </Typography>
              </Box>
              <Box
                sx={{
                  mt: 2,
                  display: "grid",
                  gridTemplateColumns: "1fr 1fr",
                  gap: 1,
                }}
              >
                <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                  <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                    CPU
                  </Typography>
                  <Typography sx={{ fontFamily: "monospace", color: "#2196f3", fontSize: "0.7rem", fontWeight: 600 }}>
                    {info.fastfetch.cpu_model}
                  </Typography>
                </Box>
                <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                  <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                    SHELL
                  </Typography>
                  <Typography sx={{ fontFamily: "monospace", color: "#2196f3", fontSize: "0.7rem", fontWeight: 600 }}>
                    {info.fastfetch.shell}
                  </Typography>
                </Box>
                <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                  <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                    PACKAGES
                  </Typography>
                  <Typography sx={{ fontFamily: "monospace", color: "#2196f3", fontSize: "0.9rem", fontWeight: 700 }}>
                    {info.fastfetch.packages}
                  </Typography>
                </Box>
                <Box sx={{ p: 1.5, border: "1px solid rgba(255,255,255,0.05)", borderRadius: 1 }}>
                  <Typography variant="caption" sx={{ color: "#666", fontSize: "0.6rem", display: "block" }}>
                    GPU STATUS
                  </Typography>
                  <Typography
                    sx={{
                      fontFamily: "monospace",
                      color: info.nvtop.gpu_status === "Accelerating" ? "#4caf50" : "#888",
                      fontSize: "0.9rem",
                      fontWeight: 700,
                    }}
                  >
                    {info.nvtop.gpu_status.toUpperCase()}
                  </Typography>
                </Box>
              </Box>
            </Box>
          </Paper>

          {/* Logs / Activity */}
          <Paper
            sx={{
              p: 2.5,
              bgcolor: "rgba(0,0,0,0.3)",
              border: "1px solid rgba(255,255,255,0.06)",
            }}
          >
            <SectionHeader
              icon={<TerminalIcon fontSize="small" />}
              title="HEALTH LOG"
              color="#ffb74d"
              badge="LIVE"
            />
            <Box
              sx={{
                mt: 1,
                fontFamily: "monospace",
                fontSize: "0.7rem",
                color: "#aaa",
                maxHeight: 260,
                overflowY: "auto",
              }}
            >
              <Box sx={{ color: "#666", mb: 0.8 }}>
                [{new Date().toISOString().replace("T", " ").slice(0, 19)}] System monitor initialized
              </Box>
              {info.nvtop.gpu_status === "Accelerating" && (
                <Box sx={{ color: "#ce93d8", mb: 0.8 }}>
                  GPU acceleration active — {info.nvtop.gpu_usage.toFixed(1)}% load
                </Box>
              )}
              {info.nvtop.temp > 70 && (
                <Box sx={{ color: "#f44336", mb: 0.8 }}>
                  WARNING: Temperature high ({info.nvtop.temp.toFixed(0)}°C) — consider checking cooling
                </Box>
              )}
              {info.disk.percent > 85 && (
                <Box sx={{ color: "#f44336", mb: 0.8 }}>
                  WARNING: Disk usage high ({info.disk.percent.toFixed(1)}%) — {info.disk.free} GB remaining
                </Box>
              )}
              {info.htop.mem_percent > 90 && (
                <Box sx={{ color: "#f44336", mb: 0.8 }}>
                  WARNING: Memory usage critical ({info.htop.mem_percent.toFixed(1)}%)
                </Box>
              )}
              <Box sx={{ color: "#4caf50", mb: 0.8 }}>
                OK — CPU: {info.htop.cpu_total.toFixed(1)}% | RAM: {memUsedGB.toFixed(1)}G | Disk: {info.disk.free}G free
              </Box>
              <Box sx={{ color: "#666", mb: 0.8 }}>
                Kernel: {info.fastfetch.kernel.split(" ")[0]} | Host: {info.fastfetch.host} | Uptime: {info.fastfetch.uptime}
              </Box>
              <Box sx={{ color: "#4db6ac", mb: 0.8 }}>
                Accelerator: {info.accelerator}
              </Box>
              <Box sx={{ color: "#ffb74d", mb: 0.8 }}>
                Refresh interval: 2s — {info.htop.cpu_cores.length} CPU cores monitored
              </Box>
              {info.nvtop.vram_total > 0 && (
                <Box sx={{ color: "#7b1fa2", mb: 0.8 }}>
                  VRAM: {info.nvtop.vram_used}M / {info.nvtop.vram_total}M
                </Box>
              )}
            </Box>
          </Paper>
        </Box>

        {/* Footer */}
        <Box
          sx={{
            mt: 2,
            p: 1.5,
            border: "1px solid rgba(255,255,255,0.04)",
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            flexWrap: "wrap",
            gap: 1,
          }}
        >
          <Typography variant="caption" sx={{ fontFamily: "monospace", color: "#444", fontSize: "0.65rem" }}>
            MOONSHADOW NVR — AI ARCHITECTURE ENABLED
          </Typography>
          <Box sx={{ display: "flex", gap: 1 }}>
            <Chip
              label={`CPU: ${info.htop.cpu_total.toFixed(0)}%`}
              size="small"
              sx={{ height: 18, fontSize: "0.6rem", fontFamily: "monospace", bgcolor: "rgba(33,150,243,0.1)", color: "#2196f3" }}
            />
            <Chip
              label={`GPU: ${info.nvtop.gpu_usage.toFixed(0)}%`}
              size="small"
              sx={{ height: 18, fontSize: "0.6rem", fontFamily: "monospace", bgcolor: "rgba(206,147,216,0.1)", color: "#ce93d8" }}
            />
            <Chip
              label={`TEMP: ${info.nvtop.temp.toFixed(0)}°C`}
              size="small"
              sx={{ height: 18, fontSize: "0.6rem", fontFamily: "monospace", bgcolor: "rgba(255,152,0,0.1)", color: tempColor(info.nvtop.temp) }}
            />
            <Chip
              label={`DISK: ${info.disk.percent.toFixed(0)}%`}
              size="small"
              sx={{ height: 18, fontSize: "0.6rem", fontFamily: "monospace", bgcolor: "rgba(255,152,0,0.1)", color: "#ff9800" }}
            />
          </Box>
        </Box>
      </Container>
    </Frame>
  );
}