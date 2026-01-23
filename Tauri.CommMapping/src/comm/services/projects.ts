import type {
  CommProjectCopyRequest,
  CommProjectCreateRequest,
  CommProjectDataV1,
  CommProjectsListRequest,
  CommProjectsListResponse,
  CommProjectV1,
  CommProjectUiStateV1,
} from "../api";
import {
  commProjectCopy,
  commProjectCreate,
  commProjectDelete,
  commProjectLoadV1,
  commProjectSaveV1,
  commProjectUiStatePatchV1,
  commProjectsList,
} from "../api";

export async function listProjects(
  request?: CommProjectsListRequest
): Promise<CommProjectsListResponse> {
  return commProjectsList(request);
}

export async function createProject(
  request: CommProjectCreateRequest
): Promise<CommProjectV1> {
  return commProjectCreate(request);
}

export async function copyProject(
  request: CommProjectCopyRequest
): Promise<CommProjectV1> {
  return commProjectCopy(request);
}

export async function deleteProject(projectId: string): Promise<CommProjectV1> {
  return commProjectDelete(projectId);
}

export async function loadProjectData(projectId: string): Promise<CommProjectDataV1> {
  return commProjectLoadV1(projectId);
}

export async function saveProjectData(payload: CommProjectDataV1): Promise<void> {
  return commProjectSaveV1(payload);
}

export async function patchProjectUiState(projectId: string, patch: CommProjectUiStateV1): Promise<void> {
  return commProjectUiStatePatchV1(projectId, patch);
}
