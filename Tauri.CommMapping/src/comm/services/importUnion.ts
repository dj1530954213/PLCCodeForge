import type { CommImportUnionXlsxResponse, ImportUnionOptions } from "../api";
import { commImportUnionXlsx } from "../api";

export async function importUnionXlsx(
  path: string,
  options?: ImportUnionOptions
): Promise<CommImportUnionXlsxResponse> {
  return commImportUnionXlsx(path, options);
}
