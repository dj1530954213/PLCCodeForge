import { computed, ref, watch } from "vue";
import type {
  CommDeviceCopyRuleV1,
  CommDeviceCopyTemplateV1,
  CommDeviceV1,
  CommProjectDataV1,
  ConnectionProfile,
} from "../api";
import { useCommDeviceContext } from "./useDeviceContext";
import { newPointKey } from "../services/ids";
import {
  confirmAction,
  notifyError,
  notifySuccess,
  notifyWarning,
  resolveErrorMessage,
} from "../services/notify";
import { cloneProfile } from "../services/profiles";

export function useDevicePanel() {
  const { project, devices, activeDeviceId, activeDevice, saveProject } = useCommDeviceContext();

  const deviceEdit = ref({ name: "", workbookName: "" });
  const deviceDirty = computed(() => {
    const current = activeDevice.value;
    if (!current) return false;
    return (
      deviceEdit.value.name.trim() !== current.deviceName ||
      deviceEdit.value.workbookName.trim() !== current.workbookName
    );
  });

  const addDialogOpen = ref(false);
  const addDeviceName = ref("");
  const addUseActiveProfile = ref(true);

  const copyDialogOpen = ref(false);
  const copySourceDeviceId = ref("");
  const copyDeviceName = ref("");
  const copyRules = ref<CommDeviceCopyRuleV1[]>([]);
  const copyTemplateId = ref("");
  const copyTemplateName = ref("");

  const copyTemplates = computed<CommDeviceCopyTemplateV1[]>(() => {
    return project.value?.uiState?.deviceCopyTemplates ?? [];
  });

  function sanitizeWorkbookName(name: string): string {
    const sanitized = name.replace(/[\\/:*?"<>|]/g, "_").trim();
    return sanitized ? sanitized : "Device";
  }

  function defaultProfile(): ConnectionProfile {
    return {
      protocolType: "TCP",
      channelName: "tcp-1",
      deviceId: 1,
      readArea: "Holding",
      startAddress: 0,
      length: 1,
      ip: "127.0.0.1",
      port: 502,
      timeoutMs: 1000,
      retryCount: 0,
      pollIntervalMs: 500,
    };
  }

  function normalizedCopyRules(): CommDeviceCopyRuleV1[] {
    return copyRules.value
      .map((r) => ({ find: r.find.trim(), replace: r.replace ?? "" }))
      .filter((r) => r.find.length > 0);
  }

  function applyCopyRules(value: string, rules: CommDeviceCopyRuleV1[]): string {
    let out = value;
    for (const rule of rules) {
      if (!rule.find) continue;
      out = out.split(rule.find).join(rule.replace);
    }
    return out;
  }

  function resetCopyRules() {
    copyRules.value = [{ find: "", replace: "" }];
    copyTemplateId.value = "";
    copyTemplateName.value = "";
  }

  function selectDevice(deviceId: string) {
    activeDeviceId.value = deviceId;
  }

  function openAddDialog() {
    addDeviceName.value = "";
    addUseActiveProfile.value = true;
    addDialogOpen.value = true;
  }

  function openCopyDialog() {
    const active = activeDevice.value;
    if (!active) return;
    copySourceDeviceId.value = active.deviceId;
    copyDeviceName.value = `${active.deviceName}-copy`;
    resetCopyRules();
    copyDialogOpen.value = true;
  }

  async function confirmDeleteDevice() {
    const current = project.value;
    const active = activeDevice.value;
    if (!current || !active) {
      notifyWarning("未选择设备");
      return;
    }

    const ok = await confirmAction(
      `确认删除设备「${active.deviceName}」？删除后将无法恢复该设备的点位与连接配置。`,
      "删除设备",
      {
        confirmButtonText: "删除",
        cancelButtonText: "取消",
        type: "warning",
      }
    );
    if (!ok) return;

    const nextDevices = (current.devices ?? []).filter((d) => d.deviceId !== active.deviceId);
    const nextActiveId = nextDevices[0]?.deviceId ?? "";
    const next: CommProjectDataV1 = {
      ...current,
      devices: nextDevices,
      uiState: {
        ...(current.uiState ?? {}),
        activeDeviceId: nextActiveId,
      },
    };

    try {
      await saveProject(next);
      notifySuccess("设备已删除");
    } catch (e: unknown) {
      notifyError(resolveErrorMessage(e, "删除设备失败"));
    }
  }

  async function confirmAddDevice() {
    const name = addDeviceName.value.trim();
    if (!name) {
      notifyError("设备名称不能为空");
      return;
    }
    const current = project.value;
    if (!current) return;

    const deviceId = newPointKey();
    const profile = addUseActiveProfile.value && activeDevice.value
      ? cloneProfile(activeDevice.value.profile)
      : defaultProfile();

    const newDevice: CommDeviceV1 = {
      deviceId,
      deviceName: name,
      workbookName: sanitizeWorkbookName(name),
      profile,
      points: { schemaVersion: 1, points: [] },
    };

    const next: CommProjectDataV1 = {
      ...current,
      devices: [...(current.devices ?? []), newDevice],
      uiState: {
        ...(current.uiState ?? {}),
        activeDeviceId: deviceId,
      },
    };

    try {
      await saveProject(next);
      activeDeviceId.value = deviceId;
      addDialogOpen.value = false;
      notifySuccess("已新增设备");
    } catch (e: unknown) {
      notifyError(resolveErrorMessage(e, "新增设备失败"));
    }
  }

  async function confirmCopyDevice() {
    const name = copyDeviceName.value.trim();
    if (!name) {
      notifyError("设备名称不能为空");
      return;
    }
    const current = project.value;
    if (!current) return;
    const source = (current.devices ?? []).find((d) => d.deviceId === copySourceDeviceId.value);
    if (!source) {
      notifyError("源设备不存在");
      return;
    }

    const rules = normalizedCopyRules();
    const copiedPoints = source.points.points.map((p) => ({
      ...p,
      pointKey: newPointKey(),
      hmiName: applyCopyRules(p.hmiName, rules),
    }));

    const deviceId = newPointKey();
    const newDevice: CommDeviceV1 = {
      deviceId,
      deviceName: name,
      workbookName: sanitizeWorkbookName(name),
      profile: cloneProfile(source.profile),
      points: {
        schemaVersion: source.points.schemaVersion,
        points: copiedPoints,
      },
    };

    const next: CommProjectDataV1 = {
      ...current,
      devices: [...(current.devices ?? []), newDevice],
      uiState: {
        ...(current.uiState ?? {}),
        activeDeviceId: deviceId,
      },
    };

    try {
      await saveProject(next);
      activeDeviceId.value = deviceId;
      copyDialogOpen.value = false;
      notifySuccess("已复制设备");
    } catch (e: unknown) {
      notifyError(resolveErrorMessage(e, "复制设备失败"));
    }
  }

  async function saveDeviceMeta() {
    const current = project.value;
    const active = activeDevice.value;
    if (!current || !active) {
      notifyError("未选择设备");
      return;
    }
    const name = deviceEdit.value.name.trim();
    if (!name) {
      notifyError("设备名称不能为空");
      return;
    }
    const workbookName = deviceEdit.value.workbookName.trim() || sanitizeWorkbookName(name);
    const devices = current.devices ?? [];
    const idx = devices.findIndex((d) => d.deviceId === active.deviceId);
    if (idx < 0) return;
    const nextDevices = [...devices];
    nextDevices[idx] = {
      ...nextDevices[idx],
      deviceName: name,
      workbookName,
    };
    const next: CommProjectDataV1 = {
      ...current,
      devices: nextDevices,
    };
    await saveProject(next);
    notifySuccess("设备信息已保存");
  }

  async function saveCopyTemplate() {
    const name = copyTemplateName.value.trim();
    if (!name) {
      notifyError("模板名称不能为空");
      return;
    }
    const rules = normalizedCopyRules();
    if (rules.length === 0) {
      notifyError("至少需要一条替换规则");
      return;
    }
    const current = project.value;
    if (!current) return;

    const nextTemplate: CommDeviceCopyTemplateV1 = {
      templateId: newPointKey(),
      name,
      rules,
    };
    const list = current.uiState?.deviceCopyTemplates ?? [];
    const next: CommProjectDataV1 = {
      ...current,
      uiState: {
        ...(current.uiState ?? {}),
        deviceCopyTemplates: [...list, nextTemplate],
      },
    };

    try {
      await saveProject(next);
      copyTemplateId.value = nextTemplate.templateId;
      notifySuccess("模板已保存");
    } catch (e: unknown) {
      notifyError(resolveErrorMessage(e, "保存模板失败"));
    }
  }

  async function deleteCopyTemplate() {
    const current = project.value;
    if (!current) return;
    const id = copyTemplateId.value;
    if (!id) return;

    const list = current.uiState?.deviceCopyTemplates ?? [];
    const nextList = list.filter((t) => t.templateId !== id);
    const next: CommProjectDataV1 = {
      ...current,
      uiState: {
        ...(current.uiState ?? {}),
        deviceCopyTemplates: nextList,
      },
    };

    try {
      await saveProject(next);
      copyTemplateId.value = "";
      notifySuccess("模板已删除");
    } catch (e: unknown) {
      notifyError(resolveErrorMessage(e, "删除模板失败"));
    }
  }

  watch(copyTemplateId, (id) => {
    if (!id) return;
    const template = copyTemplates.value.find((t) => t.templateId === id);
    if (!template) return;
    copyRules.value = template.rules.map((r) => ({ find: r.find, replace: r.replace }));
    copyTemplateName.value = template.name;
  });

  watch([project, activeDevice], ([nextProject, nextDevice]) => {
    if (!nextProject) {
      deviceEdit.value = { name: "", workbookName: "" };
      return;
    }
    deviceEdit.value = {
      name: nextDevice?.deviceName ?? "",
      workbookName: nextDevice?.workbookName ?? "",
    };
  });

  return {
    devices,
    activeDeviceId,
    activeDevice,
    deviceEdit,
    deviceDirty,
    addDialogOpen,
    addDeviceName,
    addUseActiveProfile,
    copyDialogOpen,
    copySourceDeviceId,
    copyDeviceName,
    copyRules,
    copyTemplateId,
    copyTemplateName,
    copyTemplates,
    sanitizeWorkbookName,
    selectDevice,
    openAddDialog,
    openCopyDialog,
    confirmDeleteDevice,
    confirmAddDevice,
    confirmCopyDevice,
    saveDeviceMeta,
    saveCopyTemplate,
    deleteCopyTemplate,
  };
}
