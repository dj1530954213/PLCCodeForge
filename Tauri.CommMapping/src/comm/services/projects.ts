import type {
  CommProjectCopyRequest,
  CommProjectCreateRequest,
  CommProjectsListRequest,
  CommProjectsListResponse,
  CommProjectV1,
} from "../api";
import {
  commProjectCopy,
  commProjectCreate,
  commProjectDelete,
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
