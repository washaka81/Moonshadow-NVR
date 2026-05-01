// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2021 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

/**
 * @file Types from the Moonshadow NVR API.
 * See descriptions in <tt>ref/api.md</tt>.
 */

export type StreamType = "main" | "sub" | "ext";

export interface Session {
  csrf: string;
}

export interface Camera {
  uuid: string;
  id: number;
  shortName: string;
  description: string;
  config?: {
    onvifBaseUrl?: string;
    username?: string;
    password?: string;
  };
  streams: Partial<Record<StreamType, Stream>>;
}

export interface Stream {
  camera: Camera; // back-reference added within api.ts.
  id: number;
  streamType: StreamType; // likewise.
  retainBytes: number;
  minStartTime90k: number;
  maxEndTime90k: number;
  totalDuration90k: number;
  totalSampleFileBytes: number;
  fsBytes: number;
  days: Record<string, Day>;
  record: boolean;
  config?: {
    mode: string;
    url?: string;
    rtspTransport: string;
    retainBytes: number;
  };
}

export interface Day {
  totalDuration90k: number;
  startTime90k: number;
  endTime90k: number;
}

export interface AiEvent {
  time_90k: number;
  camera_id: number;
  type_: string;
  value: string;
  video_link?: string;
}
