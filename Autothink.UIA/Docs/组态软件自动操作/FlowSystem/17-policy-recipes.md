# 策略配方（Policy Recipes）

## 通用重试
```yaml
policy:
  retry:
    times: 3
    intervalMs: 200
```

## 指数退避重试（建议）
```yaml
policy:
  retry:
    times: 5
    intervalMs: 200
    backoff: 1.5
```

## 剪贴板失败回退输入
```yaml
- action: try
  args:
    do:
      - action: set_clipboard
        args: { text: "@text" }
      - action: send_keys
        args: { keys: "CTRL+V" }
    fallback:
      - action: set_text
        args: { selector: "editor", text: "@text", mode: "Replace" }
```

## 弹窗处理
```yaml
- action: if
  args:
    condition: '@{exists("popupDialog")}'
    then:
      - action: click_at
        args: { anchor: "popup", pos: "okButton" }
```

## 预检查失败降级
```yaml
- action: try
  args:
    do:
      - action: ensure_selector
        args: { selector: "importDialog" }
    fallback:
      - action: log
        args: { message: "Import dialog not found, skip import" }
```

## 坐标点击失败回退
```yaml
- action: try
  args:
    do:
      - action: click_at
        args: { anchor: "mfcPanel", pos: "baudRateField" }
    fallback:
      - action: click_rel
        args: { anchor: "mfcPanel", xRatio: 0.35, yRatio: 0.22 }
```

## 任务级失败继续
```yaml
tasks:
  - id: add_program_blocks
    policy:
      onFail: warn
```
