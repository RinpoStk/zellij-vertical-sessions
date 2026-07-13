# zellij-vertical-sessions

[English](README.md)

一个轻量的 Zellij 插件，在侧边栏中垂直显示会话列表，参考[Vertical Tab Bar for Zellij](https://github.com/cfal/zellij-vertical-tabs)。

## 注意

本项目完全使用AI生成，请自行审计😁。

## 权限文件（必需）

启动插件前，需要先在 Zellij 的 `permissions.kdl` 中写入授权。

- macOS：`~/Library/Caches/org.Zellij-Contributors.Zellij/permissions.kdl`
- Linux：`~/.cache/zellij/permissions.kdl`

如果目录或文件不存在，请先创建。添加以下内容，并将路径改为 WASM 文件的实际绝对路径。

```kdl
"<插件位置>" {
    ChangeApplicationState
    ReadApplicationState
}
```

## 安装

将wasm插件复制到 `~/.config/zellij/plugins`，`examples/` 下的布局文件复制到 `~/.config/zellij/layouts/`。

## 使用

使用示例布局启动 Zellij：

```sh
zellij --layout vertical-sessions-left
# 或
zellij --layout vertical-sessions-right
```

在 Session 模式下，使用 `Up` / `Down` 选择会话，按 `Enter` 切换；也可以使用鼠标点击或滚轮。

布局及样式配置可参考 [`examples/`](examples/)。
