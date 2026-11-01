import type en from "../en";
import common from "./common";
import nav from "./nav";
import language from "./language";
import shell from "./shell";
import settings from "./settings";
import service from "./service";
import devices from "./devices";
import lcd from "./lcd";
import fans from "./fans";
import rgb from "./rgb";
import editor from "./editor";
import findings from "./findings";
import backend from "./backend";
import dashboard from "./dashboard";

const messages: typeof en = {
  common,
  nav,
  language,
  shell,
  settings,
  service,
  devices,
  lcd,
  fans,
  rgb,
  editor,
  findings,
  backend,
  dashboard,
};

export default messages;
