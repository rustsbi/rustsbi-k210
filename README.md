# RustSBI K210 平台支持包

这个平台支持包包含较多的平台兼容功能，允许在K210上运行1.12版本标准的操作系统内核。

## 二进制包下载

请参阅发行页面：[这里](https://github.com/rustsbi/rustsbi-k210/releases)。

## 使用说明

请先下载[ktool.py](https://github.com/loboris/ktool)，放置在`xtask`目录下，即文件位置为`xtask/ktool.py`。

运行以下指令，来直接在目标开发板运行代码。

```
cargo k210
```

这个平台支持包会启动位于`0x80020000`的操作系统内核，并在`a1`寄存器提供一个简单的设备树。
操作系统内核应当使用《RISC-V指令集架构 第二卷：特权级指令》的1.12版本，而非芯片支持的1.9.1版本。

## 兼容性使用文档

稍后放出。包括`sfence.vma`指令、页异常编号转发等等。

## 立即体验

先下载代码，然后直接运行内核测试：

```
cargo test
```

## 版权声明

项目的测试框架使用了[KTool](https://github.com/loboris/ktool)。这个项目使用Apache 2.0协议开源，感谢KTool项目和它的维护者们！

Reference implementaion K210 includes Kendryte K210 DTS file from Western Digital, this file is
(C) Western Digital Corporation or its affiliates under BSD-2-Clause license.
