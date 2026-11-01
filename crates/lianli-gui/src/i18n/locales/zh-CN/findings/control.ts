import type en from "../../en/findings/control";

const lingeringTitle = "Distrobox 系统启动";
const moduleTitle = "可选显示模块：{module}";
const lockTitle = "进程对主机所有权锁的访问权限";
const groupCredentialsTitle = "数字用户组凭据";
const groupMembershipStaleTitle = "此进程启动后用户组成员身份已更改";
const groupMissingTitle = "进程缺少主机锁的数字用户组";
const lockRepair =
  "安装随附的主机 tmpfiles 规则并运行 sudo systemd-tmpfiles --create lianli.conf。如使用用户服务，请将您的用户加入主机的 lianli 用户组，注销并重新登录，然后重启所选服务。系统服务必须使用软件包提供的 lianli 账户/用户组。不要替换处于活动状态的锁。";
const lockRepairDistrobox =
  "应用主机 tmpfiles/用户组设置，注销并重新登录，然后停止并重新进入现有容器，再重启其中的用户服务。比较容器内可见的数字用户组 ID，仅名称匹配是不够的。不要创建私有锁，也不要只在容器内安装主机规则。";
const evdiLibraryTitle = "EVDI 用户空间库";
const hermesAccessTitle = "Hermes 主机渲染访问权限";
const loginStartupLogs =
  "使用 journalctl --user -u lianli-session.service -b -n 100 --no-pager 查看主机日志。此检查不会启动或更改服务。";
const loginStartupRepair = `在容器“{name}”内运行：lianli-control distrobox-service-unit --desktop-session --box '{name}'。检查输出内容，并在主机上将其安装为 ~/.config/systemd/user/lianli-session.service。在主机上运行 systemctl --user daemon-reload，然后运行 systemctl --user enable --now lianli-session.service。保持主机的进程视图和私有用户运行时视图与容器共享。自定义路径请参阅 Distrobox 指南。此捕获辅助程序不会启动硬件守护进程。${loginStartupLogs}`;
const loginStartupInvalidBoxRepair = `检测到的 Distrobox 名称无效。请按照 Distrobox 桌面自启动指南操作。${loginStartupLogs}`;
const recipeTitle = "Distrobox 服务控制需要最新的服务单元定义";
const hardwareServiceRepair =
  "检查主机服务设置和日志。安装随附的单元，然后选择一种硬件模式。已有所有者在运行时，不要启动另一个守护进程。";
const storageRepair =
  "在守护进程账户的文件系统命名空间中，检查其目录访问权限、挂载状态、可用空间和配额。按照安装指南，以预期的守护进程所有权创建缺失的状态目录。修复存储后点击“重新检查”，然后重试失败的保存或媒体准备操作。";

const messages: typeof en = {
  lingeringGuidance: {
    title: lingeringTitle,
    remediation:
      "在主机上运行 sudo loginctl enable-linger {uid}，然后点击“重新检查”。驻留（lingering）允许此账户的用户服务在登录前和注销后运行。Lian Li Linux 不会自动更改此设置。",
  },
  lingeringAccountUnknown: {
    title: lingeringTitle,
    remediation: "请核实主机系统单元所用的账户，然后按照 Distrobox 驻留（lingering）指南操作。",
  },
  mediaTool: {
    title: "{tool} 可用性",
    remediation:
      "在守护进程的运行环境中（如适用，在其 Distrobox 容器内）安装带有 ffprobe 和 libx264 的 FFmpeg。核实服务的 PATH，然后点击“重新检查”，并重新保存失败的媒体设置以重试。不需要 GPU 编码器。",
  },
  moduleUnavailable: {
    title: moduleTitle,
    remediation: "请参照桌面后端指南检查主机安装。Hyprland 不需要可选显示模块。",
  },
  moduleLoaded: {
    title: moduleTitle,
  },
  moduleLoadFailed: {
    title: moduleTitle,
    remediation: "查看主机内核日志，然后修复模块兼容性或签名问题。修复后重试桌面模式。",
  },
  moduleBuildFilesMissing: {
    title: moduleTitle,
    remediation: "缺少与主机内核匹配的构建文件。请安装内核开发包，然后按照主机软件包说明修复 DKMS 构建。",
  },
  moduleBuildIncomplete: {
    title: moduleTitle,
    remediation: "查看主机 DKMS 构建日志，并为正在运行的内核完成模块安装。",
  },
  moduleInstalled: {
    title: moduleTitle,
    remediation: "如需使用此后端，请按照主机驱动启动说明操作。",
  },
  moduleNotInstalled: {
    title: moduleTitle,
    remediation: "仅在需要其桌面后端时才安装此主机驱动。Hyprland 使用原生无头输出。",
  },
  lockAccess: { title: lockTitle, remediation: lockRepair },
  lockAccessDistrobox: { title: lockTitle, remediation: lockRepairDistrobox },
  groupCredentials: { title: groupCredentialsTitle, remediation: lockRepair },
  groupCredentialsDistrobox: { title: groupCredentialsTitle, remediation: lockRepairDistrobox },
  groupMembershipStale: { title: groupMembershipStaleTitle, remediation: lockRepair },
  groupMembershipStaleDistrobox: { title: groupMembershipStaleTitle, remediation: lockRepairDistrobox },
  groupMissing: { title: groupMissingTitle, remediation: lockRepair },
  groupMissingDistrobox: { title: groupMissingTitle, remediation: lockRepairDistrobox },
  evdiLibraryLoaded: {
    title: evdiLibraryTitle,
  },
  evdiLibraryMissing: {
    title: evdiLibraryTitle,
    remediation:
      "在守护进程运行的环境中安装 libevdi 1.14 或 1.15：Fedora 上为 lian-li-linux-evdi，Debian 或 Ubuntu 上为 libevdi0，Arch 上为 evdi-dkms。然后重启守护进程服务。",
  },
  evdiLibraryMissingDistrobox: {
    title: evdiLibraryTitle,
    remediation:
      "在容器内安装该库：distrobox-enter --name {name} -- sudo dnf install --setopt=install_weak_deps=False lian-li-linux-evdi；如果是 Debian 或 Ubuntu 容器，则安装 libevdi0。然后重启守护进程服务。容器内无法看到主机上的库。",
  },
  hermesAccessGranted: {
    title: hermesAccessTitle,
  },
  hermesAccess: {
    title: hermesAccessTitle,
    remediation:
      "在主机上安装最新的 60-lianli.rules，运行 sudo udevadm control --reload-rules，然后重启电脑。其中包含 Hermes 主机 ACL 修复。仅在 Distrobox 内安装是不够的。如果仍无法访问，请参阅安装指南中的 Hermes 权限部分。",
  },
  loginStartupUnavailable: {
    title: "无法检查桌面登录自启动",
    remediation: loginStartupRepair,
  },
  loginStartupNotInstalled: {
    title: "未安装可选的桌面登录服务",
    remediation: loginStartupRepair,
  },
  loginStartupDisabled: {
    title: "可选的桌面登录自启动已禁用",
    remediation: loginStartupRepair,
  },
  loginStartupMissing: {
    title: "桌面登录服务缺失或不可用",
    remediation: loginStartupRepair,
  },
  loginStartupOutdatedRecipe: {
    title: "桌面登录服务需要最新的单元定义",
    remediation: loginStartupRepair,
  },
  loginStartupNotEnabled: {
    title: "桌面登录服务未持久启用",
    remediation: loginStartupRepair,
  },
  loginStartupFailed: {
    title: "桌面登录服务失败",
    remediation: loginStartupRepair,
  },
  loginStartupConfigured: {
    title: "桌面登录自启动已配置",
    remediation: loginStartupRepair,
  },
  loginStartupUnavailableInvalidBox: {
    title: "无法检查桌面登录自启动",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupNotInstalledInvalidBox: {
    title: "未安装可选的桌面登录服务",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupDisabledInvalidBox: {
    title: "可选的桌面登录自启动已禁用",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupMissingInvalidBox: {
    title: "桌面登录服务缺失或不可用",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupOutdatedRecipeInvalidBox: {
    title: "桌面登录服务需要最新的单元定义",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupNotEnabledInvalidBox: {
    title: "桌面登录服务未持久启用",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupFailedInvalidBox: {
    title: "桌面登录服务失败",
    remediation: loginStartupInvalidBoxRepair,
  },
  loginStartupConfiguredInvalidBox: {
    title: "桌面登录自启动已配置",
    remediation: loginStartupInvalidBoxRepair,
  },
  switchPrerequisites: {
    title: "服务切换需要主机支持",
    remediation:
      "安装匹配的主机控制辅助程序、systemd-run 和 Polkit。如使用 Distrobox，请在设置中使用“设置主机支持”。运行桌面身份验证代理，然后点击“重新检查”。",
  },
  serviceSelection: {
    title: "主机服务选择未经验证",
    remediation:
      "启动守护进程之前，请先解决任何未完成的服务切换。对于无效或隐藏的选择记录，请按照“服务模式”指南操作并保留恢复文件。不要在容器内创建私有的选择文件。",
  },
  distroboxRecipe: {
    title: recipeTitle,
    remediation:
      "在容器“{name}”内运行：lianli-control distrobox-service-unit --box '{name}'。该命令会输出主机用户单元，但不会安装或启动它。如使用自定义路径，请通过 --distrobox-enter 传入主机可执行文件路径，通过 --binaries 传入容器内的二进制文件目录。检查输出内容，正常停止任何旧的守护进程，然后在主机上将其安装为 ~/.config/systemd/user/lianli-daemon.service，并在主机上运行 systemctl --user daemon-reload。启用之前请先按照 Distrobox 指南操作。",
  },
  distroboxRecipeInvalidBox: {
    title: recipeTitle,
    remediation: "检测到的 Distrobox 名称无效。请按照 Distrobox 指南选择现有容器并更新主机用户单元。",
  },
  distroboxRecipeUnsupported: {
    title: recipeTitle,
    remediation:
      "按照 Distrobox 指南更新主机用户单元，然后重新加载用户服务管理器。将两个应用程序的二进制文件安装在同一个容器内。更改单元之前，请先正常停止旧的守护进程。",
  },
  operationLock: {
    title: "服务控制需要最新的主机设置",
    remediation:
      "在主机上安装最新的 packaging/tmpfiles.d/lianli.conf 并运行 sudo systemd-tmpfiles --create lianli.conf，然后点击“重新检查”。",
  },
  ownershipKernel: {
    title: "内核锁所有权未经验证",
    remediation: "切换模式之前，请恢复共享的主机锁和主机桥接。文件中写入的 PID 并不能证明当前的所有权。",
  },
  ownershipProcess: {
    title: "硬件所有者的服务未经验证",
    remediation:
      "等待启动或关闭完成后点击“重新检查”。服务切换需要访问主机进程和服务 cgroup。不要仅凭 PID 编号停止进程。",
  },
  userService: { title: "用户硬件服务", remediation: hardwareServiceRepair },
  systemService: { title: "系统硬件服务", remediation: hardwareServiceRepair },
  globalUserUnverified: {
    title: "全局用户自启动未经验证",
    remediation: "选择系统模式之前，请在主机上检查全局用户服务的启用状态。",
  },
  serviceConflict: {
    title: "硬件服务选择相互冲突",
    remediation:
      "选择一种硬件服务模式。切换之前，请先处理全局用户启用和手动启动的所有者进程。不要停止其他用户的守护进程。",
  },
  globalUserEnabled: {
    title: "已为所有用户启用硬件服务",
    remediation: "请管理员禁用全局默认设置，并仅为目标用户或系统服务选择自启动。",
  },
  mediaTempStorage: { title: "临时媒体存储", remediation: storageRepair },
  stateStorage: { title: "状态目录存储", remediation: storageRepair },
  usbNodeAccess: {
    title: "USB/HID 节点权限",
    remediation:
      "应用主机 USB 规则和用户组更改，然后重新登录或重启服务。如使用 Distrobox，请停止并重新进入容器。缺少 hidraw 节点时需要修复内核或设备可见性。请检查守护进程日志中的打开失败信息。",
  },
};

export default messages;
