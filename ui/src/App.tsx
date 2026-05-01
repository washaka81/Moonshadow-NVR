// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Copyright (C) 2021 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import Container from "@mui/material/Container";
import Box from "@mui/material/Box";
import React, { useEffect, useState } from "react";
import * as api from "./api";
import Login from "./Login";
import { useSnackbars } from "./snackbars";
import ListActivity from "./List";
import { Routes, Route, Navigate } from "react-router-dom";
import LiveActivity from "./Live";
import UsersActivity from "./Users";
import ChangePassword from "./ChangePassword";
import Header from "./components/Header";
import AiEventsActivity from "./AiEvents";
import CamerasActivity from "./Cameras";
import AdminDashboard from "./Admin/Dashboard";

export type LoginState =
  | "unknown"
  | "logged-in"
  | "not-logged-in"
  | "server-requires-login"
  | "user-requested-login";

export interface FrameProps {
  activityMenuPart?: React.JSX.Element;
  children?: React.ReactNode;
}

function App() {
  const [toplevel, setToplevel] = useState<api.ToplevelResponse | null>(null);
  const [timeZoneName, setTimeZoneName] = useState<string | null>(null);
  const [fetchSeq, setFetchSeq] = useState(0);
  const [loginState, setLoginState] = useState<LoginState>("unknown");
  const [changePasswordOpen, setChangePasswordOpen] = useState<boolean>(false);
  const [error, setError] = useState<api.FetchError | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const needNewFetch = () => setFetchSeq((seq) => seq + 1);
  const snackbars = useSnackbars();

  useEffect(() => {
    const handleFsChange = () => setIsFullscreen(!!document.fullscreenElement);
    document.addEventListener("fullscreenchange", handleFsChange);
    return () =>
      document.removeEventListener("fullscreenchange", handleFsChange);
  }, []);

  const onLoginSuccess = () => {
    setLoginState("logged-in");
    needNewFetch();
  };

  const logout = async () => {
    const resp = await api.logout({ csrf: toplevel!.user!.session!.csrf }, {});
    switch (resp.status) {
      case "success":
        needNewFetch();
        break;
      case "error":
        snackbars.enqueue({ message: "Logout failed: " + resp.message });
        break;
      default:
        break;
    }
  };

  useEffect(() => {
    const abort = new AbortController();
    const doFetch = async (signal: AbortSignal, isBackground = false) => {
      const resp = await api.toplevel({ signal });
      if (resp.status === "success") {
        setError(null);
        if (!isBackground) {
          setLoginState(
            resp.response.user?.session === undefined
              ? "not-logged-in"
              : "logged-in",
          );
        }
        setToplevel(resp.response);
        setTimeZoneName(resp.response.timeZoneName);
      } else if (resp.status === "error") {
        if (resp.httpStatus === 401) {
          setLoginState("server-requires-login");
        } else if (!isBackground) {
          setError(resp);
        }
      }
    };

    doFetch(abort.signal);

    const intervalId = setInterval(() => {
      doFetch(abort.signal, true);
    }, 60000);

    return () => {
      abort.abort();
      clearInterval(intervalId);
    };
  }, [fetchSeq]);

  const Frame = ({
    activityMenuPart,
    children,
  }: FrameProps): React.JSX.Element => {
    return (
      <Box
        sx={{
          display: "flex",
          flexDirection: "column",
          height: "100vh",
          width: "100vw",
          overflow: "hidden",
          bgcolor: "background.default",
        }}
      >
        {!isFullscreen && (
          <Header
            loginState={loginState}
            onLogout={logout}
            activityMenuPart={activityMenuPart}
            toplevel={toplevel}
            setLoginState={setLoginState}
            setChangePasswordOpen={setChangePasswordOpen}
          />
        )}
        <Login
          onSuccess={onLoginSuccess}
          open={
            loginState === "server-requires-login" ||
            loginState === "user-requested-login"
          }
          handleClose={() =>
            setLoginState((s) =>
              s === "user-requested-login" ? "not-logged-in" : s,
            )
          }
        />
        {toplevel?.user !== undefined && (
          <ChangePassword
            open={changePasswordOpen}
            user={toplevel?.user}
            handleClose={() => setChangePasswordOpen(false)}
          />
        )}
        {error !== null && (
          <Container>
            <h2>Error querying server</h2>
            <pre>{error.message}</pre>
          </Container>
        )}
        <Box
          sx={{
            flex: 1,
            position: "relative",
            overflow: "hidden",
            display: "flex",
            flexDirection: "column",
          }}
        >
          {children}
        </Box>
      </Box>
    );
  };

  if (toplevel == null) return <Frame />;

  return (
    <Routes>
      <Route
        path=""
        element={<LiveActivity cameras={toplevel.cameras} Frame={Frame} />}
      />
      <Route
        path="list"
        element={
          <ListActivity
            toplevel={toplevel}
            timeZoneName={timeZoneName!}
            Frame={Frame}
          />
        }
      />
      <Route
        path="users"
        element={
          <UsersActivity Frame={Frame} csrf={toplevel!.user?.session?.csrf} />
        }
      />
      <Route path="ai-events" element={<AiEventsActivity Frame={Frame} />} />
      <Route
        path="admin"
        element={<AdminDashboard Frame={Frame} toplevel={toplevel} />}
      />
      <Route
        path="cameras"
        element={
          <CamerasActivity
            toplevel={toplevel}
            Frame={Frame}
            refetch={needNewFetch}
          />
        }
      />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

export default App;
