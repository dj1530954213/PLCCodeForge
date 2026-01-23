import type { ProfilesV1 } from "../api";
import { commProfilesLoad, commProfilesSave } from "../api";

export async function loadProfiles(projectId?: string, deviceId?: string): Promise<ProfilesV1> {
  return commProfilesLoad(projectId, deviceId);
}

export async function saveProfiles(payload: ProfilesV1, projectId?: string, deviceId?: string): Promise<void> {
  return commProfilesSave(payload, projectId, deviceId);
}
