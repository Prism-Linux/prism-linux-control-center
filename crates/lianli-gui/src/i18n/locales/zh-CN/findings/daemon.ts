import type en from "../../en/findings/daemon";

const messages: typeof en = {
  wiredIdentitySave: {
    title: "无法保存有线设备标识迁移结果",
    remediation: "修复设置或从备份恢复，然后保存或重启服务。“重新检查”显示最近一次加载的结果。",
  },
  wiredIdentity: {
    title: "有线设备设置需要处理",
    remediation: "修复设置或从备份恢复，然后保存或重启服务。“重新检查”显示最近一次加载的结果。",
  },
  configurationLoad: {
    title: "配置加载",
    remediation: "修复设置或从备份恢复，然后保存或重启服务。“重新检查”显示最近一次加载的结果。",
  },
  templateLoad: {
    title: "自定义模板加载",
    remediation: "修复设置或从备份恢复，然后保存或重启服务。“重新检查”显示最近一次加载的结果。",
  },
  wirelessInitialization: {
    title: "无线初始化失败",
    remediation:
      "检查发射器和接收器的连接、守护进程日志和 USB 权限。链路恢复前，无线风扇和水泵保持最后的转速。守护进程会自动重试并重置无响应的无线控制器。“重新检查”仅读取状态。",
  },
  wirelessTransmitterUnresponsive: {
    title: "无线发射器已停止响应",
    remediation: "请拔下无线发射器并重新插入。软件重置未能使其重新启动。在其恢复响应前，无线风扇和水泵保持最后的转速。",
  },
  wirelessReceiverUnresponsive: {
    title: "无线接收器已停止响应",
    remediation: "请拔下无线接收器并重新插入。软件重置未能使其重新启动。在其恢复响应前，无线风扇和水泵保持最后的转速。",
  },
  deviceOpenFailed: {
    title: "无法打开设备",
    remediation:
      "检查连接、USB 权限和守护进程日志。修复后，如果重试已停止，请重新连接设备或重启其服务。“重新检查”仅读取状态。",
  },
  deviceOpenMore: {
    title: "其他设备打开失败",
    remediation: "其余失败请查看守护进程日志。",
  },
  lcdInitializationFailed: {
    title: "LCD 初始化失败",
    remediation: "检查 USB 访问权限和守护进程日志。修复后，重新连接 LCD 或重启服务。“重新检查”仅读取状态。",
  },
  lcdInitializationMore: {
    title: "其他 LCD 初始化失败",
    remediation: "其余失败请查看守护进程日志。",
  },
  coolingFallback: {
    title: "散热回退已生效",
    remediation: "在“风扇”和“水冷”设置中检查温度来源，然后点击“重新检查”。",
  },
  coolingTemperatures: {
    title: "散热温度",
  },
  openrgbServer: {
    title: "OpenRGB 服务器",
    remediation:
      "释放所配置的 TCP 端口，或修复守护进程网络命名空间中的访问权限。在设置中使用“重试 OpenRGB”按已保存的设置重试，或修改端口后点击“保存”。“重新检查”仅读取状态。",
  },
  runtimeHelperUnavailable: {
    title: "运行时辅助程序不可用",
    remediation:
      "在守护进程所在目录安装或重新构建匹配版本的 lianli-control。修复不可用的存储或账户服务，然后点击“重新检查”。",
  },
};

export default messages;
