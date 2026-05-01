// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Copyright (C) 2021 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import Button from "@mui/material/Button";
import IconButton from "@mui/material/IconButton";
import Menu from "@mui/material/Menu";
import MenuItem from "@mui/material/MenuItem";
import { useColorScheme } from "@mui/material/styles";
import AccountCircle from "@mui/icons-material/AccountCircle";
import React from "react";
import { LoginState } from "./App";
import Box from "@mui/material/Box";
import Brightness2 from "@mui/icons-material/Brightness2";
import Brightness7 from "@mui/icons-material/Brightness7";
import SettingsBrightness from "@mui/icons-material/SettingsBrightness";
import Tooltip from "@mui/material/Tooltip";
import ListItemIcon from "@mui/material/ListItemIcon";
import ListItemText from "@mui/material/ListItemText";

interface Props {
  loginState: LoginState;
  requestLogin: () => void;
  logout: () => void;
  changePassword: () => void;
  activityMenuPart?: React.JSX.Element;
}

function MoonshadowMenu(props: Props) {
  const { mode, setMode } = useColorScheme();
  const [accountMenuAnchor, setAccountMenuAnchor] =
    React.useState<null | HTMLElement>(null);
  const [themeMenuAnchor, setThemeMenuAnchor] =
    React.useState<null | HTMLElement>(null);

  const handleAccountMenu = (event: React.MouseEvent<HTMLElement>) => {
    setAccountMenuAnchor(event.currentTarget);
  };
  const handleThemeMenu = (event: React.MouseEvent<HTMLElement>) => {
    setThemeMenuAnchor(event.currentTarget);
  };

  const handleClose = () => {
    setAccountMenuAnchor(null);
    setThemeMenuAnchor(null);
  };

  const handleLogout = () => {
    handleClose();
    props.logout();
  };
  const handleChangePassword = () => {
    handleClose();
    props.changePassword();
  };

  const handleSetTheme = (newMode: "light" | "dark" | "system") => {
    setMode(newMode);
    handleClose();
  };

  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
      {props.activityMenuPart}

      {/* Theme Switcher Button */}
      <Tooltip title="Switch Theme">
        <IconButton
          onClick={handleThemeMenu}
          color="inherit"
          size="small"
          sx={{ opacity: 0.8 }}
        >
          {mode === "light" ? (
            <Brightness7 fontSize="small" />
          ) : mode === "dark" ? (
            <Brightness2 fontSize="small" />
          ) : (
            <SettingsBrightness fontSize="small" />
          )}
        </IconButton>
      </Tooltip>
      <Menu
        anchorEl={themeMenuAnchor}
        open={Boolean(themeMenuAnchor)}
        onClose={handleClose}
        disableScrollLock
      >
        <MenuItem onClick={() => handleSetTheme("light")}>
          <ListItemIcon>
            <Brightness7 fontSize="small" />
          </ListItemIcon>
          <ListItemText>Light</ListItemText>
        </MenuItem>
        <MenuItem onClick={() => handleSetTheme("dark")}>
          <ListItemIcon>
            <Brightness2 fontSize="small" />
          </ListItemIcon>
          <ListItemText>Dark</ListItemText>
        </MenuItem>
        <MenuItem onClick={() => handleSetTheme("system")}>
          <ListItemIcon>
            <SettingsBrightness fontSize="small" />
          </ListItemIcon>
          <ListItemText>System (Auto)</ListItemText>
        </MenuItem>
      </Menu>

      {props.loginState !== "logged-in" ? (
        <Button
          variant="outlined"
          color="inherit"
          size="small"
          onClick={props.requestLogin}
          sx={{
            textTransform: "none",
            borderColor: "rgba(255,255,255,0.2)",
            px: 2,
            height: 32,
          }}
        >
          Login
        </Button>
      ) : (
        <>
          <IconButton onClick={handleAccountMenu} color="inherit" size="small">
            <AccountCircle fontSize="small" />
          </IconButton>
          <Menu
            anchorEl={accountMenuAnchor}
            open={Boolean(accountMenuAnchor)}
            onClose={handleClose}
            anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
            transformOrigin={{ vertical: "top", horizontal: "right" }}
            disableScrollLock
          >
            <MenuItem
              onClick={handleChangePassword}
              sx={{ fontSize: "0.85rem" }}
            >
              Change Password
            </MenuItem>
            <MenuItem onClick={handleLogout} sx={{ fontSize: "0.85rem" }}>
              Logout
            </MenuItem>
          </Menu>
        </>
      )}
    </Box>
  );
}

export default MoonshadowMenu;
