import { computed, ref } from "vue";
import type { CommProjectV1 } from "../api";
import {
  copyProject as copyProjectService,
  createProject as createProjectService,
  deleteProject as deleteProjectService,
  listProjects,
} from "../services/projects";
import {
  confirmAction,
  notifyError,
  promptText,
  resolveErrorMessage,
} from "../services/notify";

type CreateProjectForm = {
  name: string;
  device: string;
  notes: string;
};

export function useProjectCatalog(options?: { includeDeleted?: boolean }) {
  const loading = ref(false);
  const showDeleted = ref(options?.includeDeleted ?? false);
  const projects = ref<CommProjectV1[]>([]);
  const createForm = ref<CreateProjectForm>({ name: "", device: "", notes: "" });

  const activeCount = computed(() => projects.value.filter((p) => !p.deletedAtUtc).length);
  const deletedCount = computed(() => projects.value.filter((p) => p.deletedAtUtc).length);

  async function refresh(includeDeleted?: boolean) {
    loading.value = true;
    try {
      const resp = await listProjects({
        includeDeleted: includeDeleted ?? showDeleted.value,
      });
      projects.value = resp.projects;
    } finally {
      loading.value = false;
    }
  }

  function resetCreateForm() {
    createForm.value = { name: "", device: "", notes: "" };
  }

  async function createProject(): Promise<CommProjectV1 | null> {
    const name = createForm.value.name.trim();
    if (!name) {
      notifyError("工程名称不能为空");
      return null;
    }
    const device = createForm.value.device.trim();
    const notes = createForm.value.notes.trim();
    try {
      return await createProjectService({
        name,
        device: device ? device : undefined,
        notes: notes ? notes : undefined,
      });
    } catch (e: unknown) {
      notifyError(resolveErrorMessage(e, "创建工程失败"));
      return null;
    }
  }

  async function copyProject(project: CommProjectV1): Promise<CommProjectV1 | null> {
    const suggested = `${project.name} (copy)`;
    const name = await promptText("输入复制后的工程名称", "复制工程", {
      inputValue: suggested,
      confirmButtonText: "复制",
      cancelButtonText: "取消",
    });

    if (!name?.trim()) {
      return null;
    }

    try {
      return await copyProjectService({ projectId: project.projectId, name: name.trim() });
    } catch (e: unknown) {
      notifyError(resolveErrorMessage(e, "复制工程失败"));
      return null;
    }
  }

  async function deleteProject(project: CommProjectV1): Promise<boolean> {
    const ok = await confirmAction(
      `确认删除工程「${project.name}」？（软删，可通过“显示已删除”查看）`,
      "删除工程",
      {
        confirmButtonText: "删除",
        cancelButtonText: "取消",
        type: "warning",
      }
    );
    if (!ok) return false;
    try {
      await deleteProjectService(project.projectId);
      return true;
    } catch (e: unknown) {
      notifyError(resolveErrorMessage(e, "删除工程失败"));
      return false;
    }
  }

  return {
    loading,
    showDeleted,
    projects,
    createForm,
    activeCount,
    deletedCount,
    refresh,
    resetCreateForm,
    createProject,
    copyProject,
    deleteProject,
  };
}
