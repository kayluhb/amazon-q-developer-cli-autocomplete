import { GetPlatformInfoResponse } from "@tab/proto/fig";
import { sendGetPlatformInfoRequest } from "./requests.js";
import {
  AppBundleType,
  DesktopEnvironment,
  DisplayServerProtocol,
  Os,
} from "@tab/proto/fig";

export { AppBundleType, DesktopEnvironment, DisplayServerProtocol, Os };

export function getPlatformInfo(): Promise<GetPlatformInfoResponse> {
  return sendGetPlatformInfoRequest({});
}
