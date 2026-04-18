// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2025 Moonshadow NVR Contributors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

import Button from "@mui/material/Button";
import Dialog from "@mui/material/Dialog";
import DialogActions from "@mui/material/DialogActions";
import DialogContent from "@mui/material/DialogContent";
import DialogTitle from "@mui/material/DialogTitle";
import { useEffect, useState } from "react";
import * as api from "../api";
import { Camera } from "../types";
import { useSnackbars } from "../snackbars";
import React from "react";

interface Props {
  cameraToDelete?: Camera;
  csrf?: string;
  onClose: () => void;
  refetch: () => void;
}

export default function DeleteDialog({
  cameraToDelete,
  csrf,
  onClose,
  refetch,
}: Props): React.JSX.Element {
  const [req, setReq] = useState<undefined | string>();
  const snackbars = useSnackbars();
  useEffect(() => {
    const abort = new AbortController();
    const doFetch = async (uuid: string, signal: AbortSignal) => {
      const resp = await api.deleteCamera(
        uuid,
        {
          csrf: csrf,
        },
        { signal },
      );
      setReq(undefined);
      switch (resp.status) {
        case "aborted":
          break;
        case "error":
          snackbars.enqueue({
            message: "Delete failed: " + resp.message,
          });
          break;
        case "success":
          refetch();
          onClose();
          break;
      }
    };
    if (req !== undefined) {
      doFetch(req, abort.signal);
    }
    return () => {
      abort.abort();
    };
  }, [req, csrf, snackbars, onClose, refetch]);
  return (
    <Dialog open={cameraToDelete !== undefined}>
      <DialogTitle>Delete camera {cameraToDelete?.shortName}</DialogTitle>
      <DialogContent>
        This will permanently delete the given camera. Only cameras with no
        recordings can be deleted. There's no undo!
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={req !== undefined}>
          Cancel
        </Button>
        <Button
          loading={req !== undefined}
          onClick={() => setReq(cameraToDelete?.uuid)}
          color="error"
          variant="contained"
        >
          Delete
        </Button>
      </DialogActions>
    </Dialog>
  );
}
