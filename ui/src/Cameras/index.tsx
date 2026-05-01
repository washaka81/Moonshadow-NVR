// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2025 Moonshadow NVR Contributors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import Paper from "@mui/material/Paper";
import Menu from "@mui/material/Menu";
import MenuItem from "@mui/material/MenuItem";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableContainer from "@mui/material/TableContainer";
import TableHead from "@mui/material/TableHead";
import TableRow, { TableRowProps } from "@mui/material/TableRow";
import Typography from "@mui/material/Typography";
import { useState } from "react";
import * as api from "../api";
import { Camera } from "../types";
import { FrameProps } from "../App";
import AddIcon from "@mui/icons-material/Add";
import MoreVertIcon from "@mui/icons-material/MoreVert";
import IconButton from "@mui/material/IconButton";
import DeleteDialog from "./DeleteDialog";
import AddEditDialog from "./AddEditDialog";
import React from "react";
import Button from "@mui/material/Button";
import Tooltip from "@mui/material/Tooltip";
import FileDownloadIcon from "@mui/icons-material/FileDownload";
import Dialog from "@mui/material/Dialog";
import DialogTitle from "@mui/material/DialogTitle";
import DialogContent from "@mui/material/DialogContent";
import DialogContentText from "@mui/material/DialogContentText";
import DialogActions from "@mui/material/DialogActions";

interface Props {
  toplevel: api.ToplevelResponse;
  Frame: (props: FrameProps) => React.JSX.Element;
  refetch: () => void;
}

interface RowProps extends TableRowProps {
  uuid: React.ReactNode;
  name: React.ReactNode;
  description: React.ReactNode;
  gutter?: React.ReactNode;
}

/// More menu attached to a particular camera row.
interface More {
  camera: Camera;
  anchor: HTMLElement;
}

const Row = ({ uuid, name, description, gutter, ...rest }: RowProps) => (
  <TableRow {...rest}>
    <TableCell>{uuid}</TableCell>
    <TableCell>{name}</TableCell>
    <TableCell>{description}</TableCell>
    <TableCell align="right">{gutter}</TableCell>
  </TableRow>
);

const Main = ({ toplevel, Frame, refetch }: Props) => {
  const [more, setMore] = useState<undefined | More>();
  const [cameraToEdit, setCameraToEdit] = useState<undefined | null | Camera>();
  const [deleteCamera, setDeleteCamera] = useState<undefined | Camera>();
  const [importOpen, setImportOpen] = useState(false);

  const handleImport = async () => {
    try {
      const response = await fetch("/api/cameras/reload", { method: "POST" });
      if (response.ok) {
        refetch();
        setImportOpen(false);
      }
    } catch (e) {
      console.error("Import failed:", e);
    }
  };

  return (
    <Frame>
      <TableContainer component={Paper}>
        <Table size="small">
          <TableHead>
            <Row
              uuid="UUID"
              name="Short Name"
              description="Description"
              gutter={
                <>
                  <Tooltip title="Sync cameras from TUI DB">
                    <IconButton
                      aria-label="sync"
                      onClick={() => setImportOpen(true)}
                    >
                      <FileDownloadIcon />
                    </IconButton>
                  </Tooltip>
                  <IconButton
                    aria-label="add"
                    onClick={(e) => setCameraToEdit(null)}
                  >
                    <AddIcon />
                  </IconButton>
                </>
              }
            />
          </TableHead>
          <TableBody>
            {toplevel.cameras.map((c) => (
              <Row
                key={c.uuid}
                uuid={c.uuid}
                name={c.shortName}
                description={c.description}
                gutter={
                  <IconButton
                    aria-label="more"
                    onClick={(e) =>
                      setMore({
                        camera: c,
                        anchor: e.currentTarget,
                      })
                    }
                  >
                    <MoreVertIcon />
                  </IconButton>
                }
              />
            ))}
          </TableBody>
        </Table>
      </TableContainer>
      <Menu
        anchorEl={more?.anchor}
        open={more !== undefined}
        onClose={() => setMore(undefined)}
      >
        <MenuItem
          onClick={() => {
            setCameraToEdit(more?.camera);
            setMore(undefined);
          }}
        >
          Edit
        </MenuItem>
        <MenuItem
          onClick={() => {
            setDeleteCamera(more?.camera);
            setMore(undefined);
          }}
        >
          <Typography color="error">Delete</Typography>
        </MenuItem>
      </Menu>
      {cameraToEdit !== undefined && (
        <AddEditDialog
          prior={cameraToEdit}
          refetch={refetch}
          onClose={() => setCameraToEdit(undefined)}
          csrf={toplevel.user?.session?.csrf}
        />
      )}
      <DeleteDialog
        cameraToDelete={deleteCamera}
        refetch={refetch}
        onClose={() => setDeleteCamera(undefined)}
        csrf={toplevel.user?.session?.csrf}
      />
      <Dialog open={importOpen} onClose={() => setImportOpen(false)}>
        <DialogTitle>Sync Cameras from TUI</DialogTitle>
        <DialogContent>
          <DialogContentText>
            This will sync cameras from the TUI database. Continue?
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setImportOpen(false)}>Cancel</Button>
          <Button onClick={handleImport} variant="contained" color="primary">
            Sync
          </Button>
        </DialogActions>
      </Dialog>
    </Frame>
  );
};

export default Main;
