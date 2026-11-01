import type en from "../en/findings";
import labels from "./findings/labels";
import control from "./findings/control";
import daemon from "./findings/daemon";
import gui from "./findings/gui";

const messages: typeof en = { ...labels, control, daemon, gui };

export default messages;
