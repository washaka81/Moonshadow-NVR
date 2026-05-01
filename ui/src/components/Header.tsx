// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2021 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import AppBar from "@mui/material/AppBar";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import Divider from "@mui/material/Divider";
import Drawer from "@mui/material/Drawer";
import IconButton from "@mui/material/IconButton";
import List from "@mui/material/List";
import ListItemButton from "@mui/material/ListItemButton";
import ListItemIcon from "@mui/material/ListItemIcon";
import ListItemText from "@mui/material/ListItemText";
import ListIcon from "@mui/icons-material/List";
import PeopleIcon from "@mui/icons-material/People";
import Videocam from "@mui/icons-material/Videocam";
import AssessmentIcon from "@mui/icons-material/Assessment";
import SettingsIcon from "@mui/icons-material/Settings";
import * as api from "../api";

import MoonshadowMenu from "../AppMenu";
import { useReducer } from "react";
import { LoginState } from "../App";
import { Link } from "react-router-dom";
import MenuIcon from "@mui/icons-material/Menu";
import PasswordIcon from "@mui/icons-material/Password";
import Toolbar from "@mui/material/Toolbar";
import Typography from "@mui/material/Typography";

export default function Header({
  toplevel,
  loginState,
  onLogout,
  activityMenuPart,
  setLoginState,
  setChangePasswordOpen,
}: {
  toplevel: api.ToplevelResponse | null;
  loginState: LoginState;
  onLogout: () => void;
  activityMenuPart?: React.JSX.Element;
  setLoginState: React.Dispatch<React.SetStateAction<LoginState>>;
  setChangePasswordOpen: React.Dispatch<React.SetStateAction<boolean>>;
}) {
  const [showMenu, toggleShowMenu] = useReducer((state) => !state, false);

  return (
    <>
      <AppBar position="sticky">
        <Toolbar>
          {loginState !== "server-requires-login" && (
            <IconButton
              size="large"
              edge="start"
              color="inherit"
              aria-label="menu"
              sx={{ mr: 2 }}
              onClick={toggleShowMenu}
            >
              <MenuIcon />
            </IconButton>
          )}
          <Typography
            variant="h6"
            component="div"
            sx={{ flexGrow: 1, fontWeight: 300, letterSpacing: 1 }}
          >
            Moonshadow NVR
          </Typography>
          <Box sx={{ display: { xs: "none", md: "flex" }, gap: 1, mr: 2 }}>
            <Button
              color="inherit"
              component={Link}
              to="/"
              sx={{ opacity: 0.8, "&:hover": { opacity: 1 } }}
            >
              Live
            </Button>
            <Button
              color="inherit"
              component={Link}
              to="/list"
              sx={{ opacity: 0.8, "&:hover": { opacity: 1 } }}
            >
              Archives
            </Button>
            <Button
              color="inherit"
              component={Link}
              to="/ai-events"
              sx={{ opacity: 0.8, "&:hover": { opacity: 1 } }}
            >
              AI Events
            </Button>
          </Box>
          {activityMenuPart}
          {loginState === "logged-in" ? (
            <MoonshadowMenu
              loginState={loginState}
              requestLogin={() => setLoginState("user-requested-login")}
              logout={onLogout}
              changePassword={() => setChangePasswordOpen(true)}
            />
          ) : loginState === "not-logged-in" ? (
            <Button
              color="inherit"
              onClick={() => setLoginState("user-requested-login")}
            >
              Login
            </Button>
          ) : null}
        </Toolbar>
      </AppBar>
      <Drawer open={showMenu} onClose={toggleShowMenu}>
        <Box sx={{ width: 280 }} role="presentation">
          <List>
            <ListItemButton
              key="live"
              onClick={toggleShowMenu}
              component={Link}
              to="/"
            >
              <ListItemIcon>
                <Videocam />
              </ListItemIcon>
              <ListItemText
                primary="Live Mosaic"
                secondary="Real-time multi-view"
              />
            </ListItemButton>
            <ListItemButton
              key="list"
              onClick={toggleShowMenu}
              component={Link}
              to="/list"
            >
              <ListItemIcon>
                <ListIcon />
              </ListItemIcon>
              <ListItemText
                primary="Recording Archives"
                secondary="History & timeline"
              />
            </ListItemButton>
            <ListItemButton
              key="ai-events"
              onClick={toggleShowMenu}
              component={Link}
              to="/ai-events"
            >
              <ListItemIcon>
                <AssessmentIcon />
              </ListItemIcon>
              <ListItemText
                primary="AI Smart Events"
                secondary="Detections & alerts"
              />
            </ListItemButton>
            <ListItemButton
              key="admin"
              onClick={toggleShowMenu}
              component={Link}
              to="/admin"
            >
              <ListItemIcon>
                <SettingsIcon />
              </ListItemIcon>
              <ListItemText
                primary="System Diagnostics"
                secondary="Hardware & telemetry"
              />
            </ListItemButton>
            <Divider sx={{ my: 1 }} />
            {toplevel?.permissions.readCameraConfigs && (
              <ListItemButton
                key="cameras"
                onClick={toggleShowMenu}
                component={Link}
                to="/cameras"
              >
                <ListItemIcon>
                  <Videocam />
                </ListItemIcon>
                <ListItemText primary="Camera Setup" />
              </ListItemButton>
            )}
            {toplevel?.permissions.adminUsers && (
              <ListItemButton
                key="users"
                onClick={toggleShowMenu}
                component={Link}
                to="/users"
              >
                <ListItemIcon>
                  <PeopleIcon />
                </ListItemIcon>
                <ListItemText primary="User Management" />
              </ListItemButton>
            )}
          </List>
          <Divider />
          <List>
            <ListItemButton
              key="password"
              onClick={toggleShowMenu}
              component={Link}
              to="/change-password"
            >
              <ListItemIcon>
                <PasswordIcon />
              </ListItemIcon>
              <ListItemText primary="Change Password" />
            </ListItemButton>
            <ListItemButton key="logout" onClick={onLogout}>
              <ListItemIcon>
                <PasswordIcon />
              </ListItemIcon>
              <ListItemText primary="Log out" />
            </ListItemButton>
          </List>
        </Box>
      </Drawer>
    </>
  );
}
