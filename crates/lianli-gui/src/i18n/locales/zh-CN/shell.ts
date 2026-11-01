import type en from "../en/shell";

const messages: typeof en = {
  header: {
    switchToLight: "切换到浅色模式",
    switchToDark: "切换到深色模式",
    refreshing: "正在刷新…",
    refreshNow: "立即刷新",
    saveFailed: "保存失败：{error}",
    minimizeWindow: "最小化窗口",
    toggleMaximizeWindow: "最大化或还原窗口",
    closeWindow: "关闭窗口",
  },
  sidebar: {
    connected: "已连接",
    lastSeen: "上次检测到：{summary}",
  },
  installationNotice: {
    title: "检查安装",
    intro: "部分设置检查需要处理。可随时在“安装检查”中查看。",
    waitingTitle: "等待守护进程",
    waitingBody: "打开设置以管理服务。如果守护进程正在启动，请稍候并点击“重新检查”。",
    allPass: "这些安装检查现已通过。",
    later: "稍后",
    recheck: "重新检查",
    openGuide: "打开指南",
    reviewChecks: "查看检查项",
  },
  updateNotice: {
    title: "Prism Linux Control Center {version} 现已可用",
    releaseNotes: "查看发布说明",
  },
  wirelessNotice: {
    transmitterTitle: "无线发射器已停止响应",
    transmitterBody: "请拔下无线发射器并重新插入。软件重置未能使其重新启动。在其恢复响应前，无线风扇和水泵保持最后的转速。",
    receiverTitle: "无线接收器已停止响应",
    receiverBody: "请拔下无线接收器并重新插入。软件重置未能使其重新启动。在其恢复响应前，无线风扇和水泵保持最后的转速。",
  },
  compatibilityNotice: {
    title: "更改已禁用",
  },
  editableName: {
    renameDevice: "重命名此设备",
    deviceName: "设备名称",
  },
  externalLink: {
    openFailedCopied: "无法打开链接：{error}。链接已复制到剪贴板。",
    openFailed: "无法打开链接：{error}。{url}",
  },
  download: {
    timedOut: "下载超时。请检查网络连接后重试",
    tooLarge: "下载内容超过 {limit} 字节",
    noBody: "无法获取下载内容",
  },
  config: {
    copiedTemplateChanged: "已复制的模板在另一个窗口中被修改。请重新加载并检查导入内容后再保存。",
    lcdNotConfirmed: "已复制的模板已保存，但 LCD 设置未能确认。请重试保存，或重新加载以检查设置。{error}",
    previousImportPending: "请先保存或重新加载之前复制的内容。",
    reloadFailed: "配置已提交，但无法重新加载以确认结果：{error}",
    reloadEmpty: "配置保存后没有返回可重新加载的配置，无法确认结果。",
  },
};

export default messages;
