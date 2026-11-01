import type en from "../../en/findings/gui";

const messages: typeof en = {
  hostUnavailable: {
    title: "主机集成不可用",
    remediation: "使用原生安装，或恢复 Distrobox 的 /run/host 集成。使用容器私有锁时，守护进程无法安全启动。",
  },
  desktopStartupFound: {
    title: "已找到打包的桌面登录自启动文件",
    remediation: "检查 lianli-session.service，源码构建请按照桌面自启动指南操作。仅桌面模式需要此辅助程序。",
  },
  desktopStartupMissing: {
    title: "尚未建立桌面登录自动启动",
    remediation: "检查 lianli-session.service，源码构建请按照桌面自启动指南操作。仅桌面模式需要此辅助程序。",
  },
  usbRules: {
    title: "打包的 USB 权限规则",
    remediation:
      "在主机上安装当前的 USB 规则，重新加载 udev 规则并重新应用。替换前请检查有意设置的自定义规则。仅检查文件无法验证设备权限。",
  },
  usbRulesCustomAccess: {
    title: "打包的 USB 权限规则",
    remediation:
      "无需替换规则。保留可正常工作的自定义规则。查看守护进程日志中的打开失败，并在更改设置后点击“重新检查”。",
  },
  hardwareOwnerUnreachable: {
    title: "无法连接硬件所有者",
    remediation:
      "等待启动或切换完成，然后点击“重新检查”。如果问题持续，请检查守护进程日志和套接字可见性。如果守护进程运行在 Distrobox 中，请在同一容器中启动 GUI，或暴露主机运行时套接字。",
  },
  daemonConnectionUnavailable: {
    title: "守护进程连接不可用",
    remediation:
      "在设置中选择一个服务，或启动手动运行的守护进程，然后点击“重新检查”。使用 Distrobox 时，GUI 和守护进程必须共享一个可见的运行时套接字。",
  },
  runtimeChecksUnavailable: {
    title: "运行时安装检查不可用",
    remediation:
      "在 GUI 和守护进程所在目录安装匹配版本的 lianli-control，启动所选服务，然后点击“重新检查”。源码构建和离线诊断请参阅故障排除指南。",
  },
  daemonLockIdentity: {
    title: "已连接守护进程的锁标识",
    remediation:
      "更新所有守护进程启动方式，正常停止旧的所有者，然后重启所选服务。不要删除正在使用的锁文件，也不要通过启动另一个守护进程来修复此问题。",
  },
  ownershipLock: {
    title: "共享守护进程所有权锁",
    remediation:
      "在主机上安装随附的 tmpfiles 规则，然后运行 sudo systemd-tmpfiles --create lianli.conf。守护进程运行期间，切勿删除或替换锁文件。",
  },
};

export default messages;
