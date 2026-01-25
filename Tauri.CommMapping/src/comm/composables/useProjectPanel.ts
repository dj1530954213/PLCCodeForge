import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import type { CommProjectDataV1, CommProjectV1 } from "../api";
import { useCommDeviceContext } from "./useDeviceContext";
import { copyProject, createProject, deleteProject, listProjects } from "../services/projects";
import { confirmAction, notifyError, notifySuccess, promptText } from "../services/notify";

export function useProjectPanel() {
  const route = useRoute();
  const router = useRouter();

  const { project, devices, activeDevice, reloadProject, saveProject } = useCommDeviceContext();

  const projectId = computed(() => String(route.params.projectId ?? ""));
  const projectList = ref<CommProjectV1[]>([]);
  const projectListLoading = ref(false);
  const projectCreateOpen = ref(false);
  const projectCreateForm = ref({ name: "", device: "", notes: "" });

  const selectedProjectId = computed<string>({
    get() {
      return projectId.value;
    },
    set(value) {
      if (!value || value === projectId.value) return;
      router.push(`/projects/${value}/comm/points`);
    },
  });

  const projectEdit = ref({ name: "", device: "", notes: "" });
  const projectDirty = computed(() => {
    const current = project.value;
    if (!current) return false;
    return (
      projectEdit.value.name.trim() !== current.name ||
      projectEdit.value.device.trim() !== (current.device ?? "") ||
      projectEdit.value.notes.trim() !== (current.notes ?? "")
    );
  });

  async function loadProjectList() {
    projectListLoading.value = true;
    try {
      const resp = await listProjects({ includeDeleted: false });
      projectList.value = resp.projects.filter((p) => !p.deletedAtUtc);
    } finally {
      projectListLoading.value = false;
    }
  }

  function openCreateProject() {
    projectCreateForm.value = { name: "", device: "", notes: "" };
    projectCreateOpen.value = true;
  }

  async function confirmCreateProject() {
    const name = projectCreateForm.value.name.trim();
    if (!name) {
      notifyError("工程名称不能为空");
      return;
    }
    const device = projectCreateForm.value.device.trim();
    const notes = projectCreateForm.value.notes.trim();
    const created = await createProject({
      name,
      device: device ? device : undefined,
      notes: notes ? notes : undefined,
    });
    projectCreateOpen.value = false;
    await loadProjectList();
    router.push(`/projects/${created.projectId}/comm/points`);
  }

  async function copyCurrentProject() {
    const current = project.value;
    if (!current) {
      notifyError("未选择工程");
      return;
    }
    const suggested = `${current.name} (copy)`;
    const name = await promptText("输入复制后的工程名称", "复制工程", {
      inputValue: suggested,
      confirmButtonText: "复制",
      cancelButtonText: "取消",
    });
    if (!name?.trim()) return;
    const created = await copyProject({ projectId: current.projectId, name: name.trim() });
    notifySuccess("已复制工程");
    await loadProjectList();
    router.push(`/projects/${created.projectId}/comm/points`);
  }

  async function deleteCurrentProject() {
    const current = project.value;
    if (!current) {
      notifyError("未选择工程");
      return;
    }
    const ok = await confirmAction(`确认删除工程「${current.name}」？（软删）`, "删除工程", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
    if (!ok) return;
    await deleteProject(current.projectId);
    notifySuccess("已删除（软删）");
    await loadProjectList();
    const next = projectList.value.find((p) => p.projectId !== current.projectId);
    if (next) {
      router.push(`/projects/${next.projectId}/comm/points`);
    } else {
      router.push("/");
    }
  }

  async function saveProjectMeta() {
    const current = project.value;
    if (!current) {
      notifyError("未选择工程");
      return;
    }
    const name = projectEdit.value.name.trim();
    if (!name) {
      notifyError("工程名称不能为空");
      return;
    }
    const next: CommProjectDataV1 = {
      ...current,
      name,
      device: projectEdit.value.device.trim() || undefined,
      notes: projectEdit.value.notes.trim() || undefined,
    };
    await saveProject(next);
    await loadProjectList();
    notifySuccess("工程信息已保存");
  }

  watch(
    projectId,
    () => {
      void loadProjectList();
    },
    { immediate: true }
  );

  watch(project, (next) => {
    if (!next) {
      projectEdit.value = { name: "", device: "", notes: "" };
      return;
    }
    projectEdit.value = {
      name: next.name ?? "",
      device: next.device ?? "",
      notes: next.notes ?? "",
    };
  });

  watch(project, (next) => {
    if (next) {
      localStorage.setItem("comm.lastProjectId", next.projectId);
    }
  });

  return {
    project,
    devices,
    activeDevice,
    projectId,
    projectList,
    projectListLoading,
    projectCreateOpen,
    projectCreateForm,
    selectedProjectId,
    projectEdit,
    projectDirty,
    loadProjectList,
    openCreateProject,
    confirmCreateProject,
    copyCurrentProject,
    deleteCurrentProject,
    reloadProject,
    saveProjectMeta,
  };
}
