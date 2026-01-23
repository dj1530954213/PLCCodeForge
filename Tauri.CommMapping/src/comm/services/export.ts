import type {
  CommExportDeliveryXlsxResponse,
  CommExportXlsxResponse,
  DeliveryResultsSource,
  PointsV1,
  ProfilesV1,
  RunStats,
  SampleResult,
} from "../api";
import { commExportDeliveryXlsx, commExportXlsx } from "../api";

export async function exportXlsx(
  request: { outPath: string; profiles?: ProfilesV1; points?: PointsV1 },
  projectId?: string,
  deviceId?: string
): Promise<CommExportXlsxResponse> {
  return commExportXlsx(request, projectId, deviceId);
}

export async function exportDeliveryXlsx(
  request: {
    outPath: string;
    includeResults?: boolean;
    resultsSource?: DeliveryResultsSource;
    results?: SampleResult[];
    stats?: RunStats;
    profiles?: ProfilesV1;
    points?: PointsV1;
  },
  projectId?: string,
  deviceId?: string
): Promise<CommExportDeliveryXlsxResponse> {
  return commExportDeliveryXlsx(request, projectId, deviceId);
}
