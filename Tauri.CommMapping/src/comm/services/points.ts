import type { PointsV1 } from "../api";
import { commPointsLoad, commPointsSave } from "../api";

export async function loadPoints(projectId?: string, deviceId?: string): Promise<PointsV1> {
  return commPointsLoad(projectId, deviceId);
}

export async function savePoints(payload: PointsV1, projectId?: string, deviceId?: string): Promise<void> {
  return commPointsSave(payload, projectId, deviceId);
}
