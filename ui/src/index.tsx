// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2021 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import { ThemeProvider, createTheme } from "@mui/material/styles";
import { LocalizationProvider } from "@mui/x-date-pickers/LocalizationProvider";
import "@fontsource/roboto";
import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import ErrorBoundary from "./ErrorBoundary";
import { SnackbarProvider } from "./snackbars";
import { AdapterDateFns } from "@mui/x-date-pickers/AdapterDateFns";
import "./index.css";
import { HashRouter } from "react-router-dom";
import CssBaseline from "@mui/material/CssBaseline";

const theme = createTheme({
  cssVariables: {
    colorSchemeSelector: "data",
  },
  colorSchemes: {
    dark: {
      palette: {
        primary: { main: "#ffffff" },
        background: { default: "#000000", paper: "#0a0a0a" },
        header: "#000000",
        headerContrastText: "#ffffff",
      },
    },
    light: {
      palette: {
        primary: { main: "#000000" },
        background: { default: "#ffffff", paper: "#ffffff" },
        header: "#ffffff",
        headerContrastText: "#000000",
      },
    },
  },
  typography: {
    fontFamily: "'Inter', 'Roboto', sans-serif",
    button: { textTransform: "none", fontWeight: 500 },
  },
  shape: { borderRadius: 4 },
  components: {
    MuiCssBaseline: {
      styleOverrides: {
        body: { backgroundColor: "#000000", color: "#eeeeee", margin: 0 },
      },
    },
    MuiButton: {
      defaultProps: { disableElevation: true },
      styleOverrides: {
        root: {
          borderColor: "rgba(255,255,255,0.2)",
          "&:hover": { borderColor: "#ffffff" },
        },
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          backgroundImage: "none",
          border: "1px solid rgba(255,255,255,0.05)",
          boxShadow: "none",
        },
      },
    },
  },
});

const container = document.getElementById("root");
const root = createRoot(container!);
root.render(
  <React.StrictMode>
    <HashRouter>
      <ThemeProvider theme={theme}>
        <CssBaseline />
        <ErrorBoundary>
          <LocalizationProvider dateAdapter={AdapterDateFns}>
            <SnackbarProvider autoHideDuration={5000}>
              <App />
            </SnackbarProvider>
          </LocalizationProvider>
        </ErrorBoundary>
      </ThemeProvider>
    </HashRouter>
  </React.StrictMode>,
);
