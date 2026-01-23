import type {
  CommRunLatestObsResponse,
  CommRunLatestResponse,
  CommRunStartObsResponse,
  CommRunStopObsResponse,
  PlanV1,
  PlanOptions,
  PointsV1,
  ProfilesV1,
  CommDriverKind,
  ReadPlan,
} from "../api";
import {
  commPlanBuild,
  commRunLatest,
  commRunLatestObs,
  commRunStartObs,
  commRunStopObs,
} from "../api";

export async function buildPlan(
  request: { profiles?: ProfilesV1; points?: PointsV1; options?: PlanOptions },
  projectId?: string,
  deviceId?: string
): Promise<PlanV1> {
  return commPlanBuild(request, projectId, deviceId);
}

export async function runStartObs(
  request: { driver?: CommDriverKind; profiles?: ProfilesV1; points?: PointsV1; plan?: ReadPlan },
  projectId?: string,
  deviceId?: string
): Promise<CommRunStartObsResponse> {
  return commRunStartObs(request, projectId, deviceId);
}

export async function runLatestObs(runId: string): Promise<CommRunLatestObsResponse> {
  return commRunLatestObs(runId);
}

export async function runStopObs(runId: string, projectId?: string): Promise<CommRunStopObsResponse> {
  return commRunStopObs(runId, projectId);
}

export async function runLatest(runId: string): Promise<CommRunLatestResponse> {
  return commRunLatest(runId);
}
