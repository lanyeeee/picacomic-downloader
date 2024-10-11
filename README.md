# 哔咔漫画下载器

<p align="center">
    <img src="https://github.com/user-attachments/assets/65ec372c-9ee9-4b09-b05e-6c939d15ac4d" width="200" style="align-self: center"/>
</p>

一个用于 manhuabika.com 哔咔漫画 pica漫画 bika漫画 PicACG 的多线程下载器，带图形界面，带收藏夹，下载速度飞快。图形界面基于[Tauri](https://v2.tauri.app/start/)

在[Release页面](https://github.com/lanyeeee/picacomic-downloader/releases)可以直接下载

# 图形界面
![image](https://github.com/user-attachments/assets/97294210-2e49-4d8c-ad8e-809bc5a619c9)

# 使用方法

1. 点击`账号登录`按钮完成登录(~~直接输入`Authorization`也行~~)
2. 使用`漫画搜索`或`漫画收藏`，选择要下载的漫画，点击后进入`章节详情`
3. 在`章节详情`勾选要下载的章节，点击`下载勾选章节`按钮开始下载
4. 下载完成后点击`打开下载目录`按钮查看结果

下面的视频是完整使用流程，**没有H内容，请放心观看**

https://github.com/user-attachments/assets/8d08bba1-851c-41e4-ba3e-66f99c7f614b

# 关于被杀毒软件误判为病毒

对于个人开发者来说，这个问题几乎是无解的(~~需要购买数字证书给软件签名，甚至给杀毒软件交保护费~~)  
我能想到的解决办法只有：

1. 根据下面的**如何构建(build)**，自行编译
2. 希望你相信我的承诺，我承诺你在[Release页面](https://github.com/lanyeeee/picacomic-downloader/releases)下载到的所有东西都是安全的

# 如何构建(build)

构建非常简单，一共就3条命令  
~~前提是你已经安装了Rust、Node、pnpm~~

#### 前提

- [Rust](https://www.rust-lang.org/tools/install)
- [Node](https://nodejs.org/en)
- [pnpm](https://pnpm.io/installation)

#### 步骤

#### 1. 克隆本仓库

```
git clone https://github.com/lanyeeee/picacomic-downloader.git
```

#### 2.安装依赖

```
cd picacomic-downloader
pnpm install
```

#### 3.构建(build)

```
pnpm tauri build
```
# 提交PR
**PR请提交至`develop`分支**

**如果想新加一个功能，请先开个`issue`或`discussion`讨论一下，避免无效工作**

其他情况的PR欢迎直接提交，比如：
1. 对原有功能的改进
2. 使用更轻量的库实现原有功能
3. 修订文档
4. 升级、更新依赖的PR也会被接受

# 免责声明
- 本工具仅作学习、研究、交流使用，使用本工具的用户应自行承担风险
- 作者不对使用本工具导致的任何损失、法律纠纷或其他后果负责
- 作者不对用户使用本工具的行为负责，包括但不限于用户违反法律或任何第三方权益的行为
# 其他
禁漫天堂下载器  
[![Readme Card](https://github-readme-stats.vercel.app/api/pin/?username=lanyeeee&repo=jmcomic-downloader)](https://github.com/lanyeeee/jmcomic-downloader)   

任何使用中遇到的问题、任何希望添加的功能，都欢迎提交issue或开discussion交流，我会尽力解决

