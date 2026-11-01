import type en from "../../en/backend/devices";

const messages: typeof en = {
  wireless: {
    imageUploadRunning: "请等待无线图片上传完成后再更改绑定",
    bindingPending: "另一个无线绑定操作正在进行中",
    deviceOffline: "设备离线",
    boundToOnlineController: "设备 {mac} 已绑定到另一个当前在线的控制器",
    deviceLimit: "最多只能绑定 10 个无线设备",
    strimerLimit: "最多只能绑定 3 个 Strimer 设备",
    lcdCLimit: "只能绑定一个 HydroShift II LCD-C",
    lcdSLimit: "只能绑定一个 HydroShift II LCD-S",
    ownedByOnlineController: "设备 {mac} 属于另一个当前在线的控制器",
    rxNotConnected: "接收器未连接",
    stopping: "无线控制器正在停止",
    notDiscovered: "未在已发现的设备中找到该设备",
    noFreeRxSlot: "没有空闲的接收器槽位：此无线控制器的所有槽位均已占用",
  },
  rgb: {
    ringRecoveryPending: "LCD 连接尚未从 RGB 上传中恢复，后续上传正在等待。",
    softwareCoolingActive: "此设备正被软件风扇或水泵控制占用，RGB 更改需等待。",
  },
  zone: {
    ring: "灯环",
    caseRing: "机箱灯环",
    screenRing: "屏幕灯环",
    ledStrip: "LED 灯带",
    pumpHead: "冷头",
    fans: "风扇",
    fan: "风扇 {number}",
    port: "端口 {number}",
    sync: "同步",
  },
  edgeHub: {
    gpu2Connector: "GPU 2 接头",
    gpu1Connector: "GPU 1 接头",
    hubAmbient: "Hub 环境温度",
    stale: "温度读数已过期",
    waiting: "正在等待温度读数",
    waitingAfterResume: "正在等待休眠恢复后的新读数",
  },
};

export default messages;
