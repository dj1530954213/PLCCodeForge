import { commSerialPortsList } from "../api";

export async function listSerialPorts(): Promise<string[]> {
  return commSerialPortsList();
}
