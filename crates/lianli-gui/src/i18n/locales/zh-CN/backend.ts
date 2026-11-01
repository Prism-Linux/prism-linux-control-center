import type en from "../en/backend";
import daemon from "./backend/daemon";
import status from "./backend/status";
import devices from "./backend/devices";
import control from "./backend/control";
import gui from "./backend/gui";

const messages: typeof en = { daemon, status, devices, control, gui };

export default messages;
