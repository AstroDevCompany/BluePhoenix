import type { AiSettingsPayload, AiStatus } from "../../lib/types";

export function settingsPayload(
  status: AiStatus,
  overrides: Partial<AiSettingsPayload> = {},
): AiSettingsPayload {
  return {
    enabled: status.enabled,
    models: status.models,
    commitFollowStyle: status.commitFollowStyle,
    setupDismissed: status.setupDismissed,
    provider: status.provider,
    localModelId: status.localModelId ?? null,
    localCtxLen: status.localCtxLen,
    localGpuOffload: status.localGpuOffload,
    localIdleUnloadMinutes: status.localIdleUnloadMinutes,
    ...overrides,
  };
}
