import type en from "../../en/backend/control";

const messages: typeof en = {
  serviceAction: {
    checkingState: "正在检查服务状态…",
    checking: "正在检查服务状态和硬件所有权…",
    authorize: "请在服务授权提示中确认以继续…",
    waiting: "正在等待服务并验证硬件所有权…",
    started: "服务已启动。守护进程身份和已加载的设置均已验证。",
    stopped: "服务已正常停止。硬件所有权已释放。",
    restarted: "服务已重启。新的守护进程身份和已加载的设置均已验证。",
    unverified:
      "无法验证服务操作是否完成：{detail}。未执行强制终止，也未自动重试。请重新检查服务，并查看所选服务的日志。",
    interrupted: "服务验证已中断。重试前请重新检查服务。",
    alreadyRunning: "另一个服务操作正在进行",
    dispatchFailed: "操作未重试。如果请求超时，请重新检查服务状态：{error}",
  },
  switch: {
    checkingPrerequisites: "正在检查服务切换的前提条件…",
    checking: "正在检查两个服务账户、启动选择和已保存的设置…",
    oldMediaUnavailable: "目标位置原有的媒体不可用。其设置将被备份，并由复制的设置替换。",
    preparing: "正在以目标账户准备设置并解码复制的媒体…",
    stoppingSource: "正在暂停竞争的启动，并正常停止当前守护进程…",
    publishing: "正在发布准备好的设置和媒体，并保留回滚备份…",
    starting: "正在启动并验证所选守护进程…",
    switched: "服务模式已切换。所选账户和已加载的配置均已验证。",
    restoring: "正在恢复之前的服务模式…",
    nothingToRecover: "没有需要恢复的服务切换。",
    archived: "已完成的切换记录已归档。",
    restored: "之前的服务模式已恢复并验证。",
    interrupted: "服务切换已中断。尝试其他模式前，请先使用“恢复中断的切换”。",
    failed:
      "服务切换失败。请在“安装检查”中重新检查。如有待恢复的切换，请选择“恢复中断的切换”。详情请查看 lianli-control-switch.service 或 lianli-control-recovery.service 的日志。",
    alreadyRunning: "另一个服务切换正在进行",
    actionRunning: "另一个服务操作正在进行",
  },
  route: {
    unsupportedContainer: "此容器中无法访问主机服务",
    hostSpawnMissing:
      "请在此容器内以交互方式运行 distrobox-host-exec true 来安装 host-spawn，然后点击“重新检查”。此检查不会安装任何辅助程序。",
    busHidden: "此容器中看不到主机用户的会话总线：{error}",
    busForeign: "可见的主机会话总线不属于此用户",
    hostToolMissing: "主机上的 /usr/bin/{name} 不可用。无法安全地执行服务检查",
    bridgeFailed: "主机命令桥接失败：{error}。请检查主机的 Flatpak 会话辅助程序，以及容器对主机会话总线的访问。",
    ownershipLockUnavailable: "主机所有权锁不可用",
    operationLockUnavailable: "主机服务操作锁不可用",
    startupPaused: "由于存在未完成的服务切换，守护进程启动已暂停。启动任一模式前，请先恢复该切换",
    ownerChanged: "检查进程期间硬件所有者发生了变化。请重新检查服务",
  },
  mediaImport: {
    identityMismatch: "导入辅助程序返回了不同的标识。重试前请检查存储",
    workerStopped: "导入进程已停止。文件可能已被复制。重试前请检查存储。",
  },
  mediaStorage: {
    noReceipt: "没有所有权凭据。已作为未验证的存储保留",
    unpublishedReceipt: "所有权凭据没有对应的已发布目录。导入可能仍在等待或已中断。删除前请检查元数据和引用。",
    invalidReceipt: "保留的所有权凭据无效：{error}",
    interruptedRemoval: "删除操作已中断。继续前请检查剩余文件和引用",
  },
};

export default messages;
