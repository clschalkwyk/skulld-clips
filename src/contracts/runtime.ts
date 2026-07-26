import type { AppError as ContractAppError } from "../../contracts/types";

export type AppError = ContractAppError;

export interface RuntimeInfo {
  appVersion: string;
  projectSchemaVersion: number;
  os: string;
  arch: string;
  ffmpegVersion: string;
  ffprobeVersion: string;
  bundledSidecars: boolean;
}
