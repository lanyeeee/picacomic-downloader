<script setup lang="ts">
import { commands } from '../bindings.ts'
import { path } from '@tauri-apps/api'
import { appDataDir } from '@tauri-apps/api/path'
import { useStore } from '../store.ts'
import { ref } from 'vue'
import { useMessage } from 'naive-ui'

const message = useMessage()

const store = useStore()

const showing = defineModel<boolean>('showing', { required: true })

const dirFmt = ref<string>(store.config?.dirFmt ?? '')

async function showConfigInFileManager() {
  const configName = 'config.json'
  const configPath = await path.join(await appDataDir(), configName)
  const result = await commands.showPathInFileManager(configPath)
  if (result.status === 'error') {
    console.error(result.error)
  }
}
</script>

<template>
  <n-modal v-model:show="showing" v-if="store.config !== undefined">
    <n-dialog class="w-140!" :showIcon="false" title="配置" @close="showing = false">
      <div class="flex flex-col gap-row-2">
        <n-radio-group v-model:value="store.config.downloadFormat">
          <span class="mr-4">下载格式</span>
          <n-tooltip placement="top" trigger="hover">
            <div>原图不为jpg时，会自动转换为jpg</div>
            <template #trigger>
              <n-radio value="Jpeg">jpg</n-radio>
            </template>
          </n-tooltip>
          <n-tooltip placement="top" trigger="hover">
            <div>原图不为png时，会自动转换为png</div>
            <template #trigger>
              <n-radio value="Png">png</n-radio>
            </template>
          </n-tooltip>
          <n-tooltip placement="top" trigger="hover">
            <div>原图不为webp时，会自动转换为webp</div>
            <template #trigger>
              <n-radio value="Webp">webp</n-radio>
            </template>
          </n-tooltip>
          <n-tooltip placement="top" trigger="hover">
            <div>
              保持原图格式，不做任何转换，
              <span class="text-red">不支持断点续传</span>
            </div>
            <template #trigger>
              <n-radio value="Original">原始格式</n-radio>
            </template>
          </n-tooltip>
        </n-radio-group>

        <div class="flex flex-col gap-2">
          <div class="flex gap-1">
            <n-input-group class="w-35%">
              <n-input-group-label size="small">章节并发数</n-input-group-label>
              <n-input-number
                class="w-full"
                v-model:value="store.config.chapterConcurrency"
                size="small"
                @update:value="message.warning('对章节并发数的修改需要重启才能生效')"
                :min="1"
                :parse="(x: string) => Number(x)" />
            </n-input-group>
            <n-input-group class="w-65%">
              <n-input-group-label size="small">每个章节下载完成后休息</n-input-group-label>
              <n-input-number
                class="w-full"
                v-model:value="store.config.chapterDownloadIntervalSec"
                size="small"
                :min="0"
                :parse="(x: string) => Number(x)" />
              <n-input-group-label size="small">秒</n-input-group-label>
            </n-input-group>
          </div>
          <div class="flex gap-1">
            <n-input-group class="w-35%">
              <n-input-group-label size="small">图片并发数</n-input-group-label>
              <n-input-number
                class="w-full"
                v-model:value="store.config.imgConcurrency"
                size="small"
                @update-value="message.warning('对图片并发数的修改需要重启才能生效')"
                :min="1"
                :parse="(x: string) => Number(x)" />
            </n-input-group>
            <n-input-group class="w-65%">
              <n-input-group-label size="small">每张图片下载完成后休息</n-input-group-label>
              <n-input-number
                class="w-full"
                v-model:value="store.config.imgDownloadIntervalSec"
                size="small"
                :min="0"
                :parse="(x: string) => Number(x)" />
              <n-input-group-label size="small">秒</n-input-group-label>
            </n-input-group>
          </div>
          <n-input-group>
            <n-input-group-label size="small">下载整个收藏夹时，每处理完一个收藏夹中的漫画后休息</n-input-group-label>
            <n-input-number
              class="w-full"
              v-model:value="store.config.downloadAllFavoritesIntervalSec"
              size="small"
              :min="0"
              :parse="(x: string) => Number(x)" />
            <n-input-group-label size="small">秒</n-input-group-label>
          </n-input-group>
        </div>

        <n-tooltip placement="top" trigger="hover" width="550">
          <div>
            可以用斜杠
            <span class="rounded bg-gray-500 px-1 text-white">/</span>
            来分隔目录层级
          </div>
          <div class="text-pink">至少要有两个层级，最后一层存放章节元数据，倒数第二层存放漫画元数据</div>
          <div class="font-semibold mt-2">可用字段：</div>
          <div class="grid grid-cols-2">
            <div>
              <span class="rounded bg-gray-500 px-1">comic_id</span>
              <span class="ml-2">漫画ID</span>
            </div>
            <div>
              <span class="rounded bg-gray-500 px-1">chapter_id</span>
              <span class="ml-2">章节ID</span>
            </div>
            <div>
              <span class="rounded bg-gray-500 px-1">comic_title</span>
              <span class="ml-2">漫画标题</span>
            </div>
            <div>
              <span class="rounded bg-gray-500 px-1">chapter_title</span>
              <span class="ml-2">章节标题</span>
            </div>
            <div>
              <span class="rounded bg-gray-500 px-1">author</span>
              <span class="ml-2">作者</span>
            </div>
            <div>
              <span class="rounded bg-gray-500 px-1">order</span>
              <span class="ml-2">章节在漫画里对应的序号</span>
            </div>
          </div>
          <div class="font-semibold mt-2">例如格式</div>
          <div class="bg-gray-200 rounded-md p-1 text-black w-fit">
            {author}/[{author}] {comic_title}({comic_id})/{order} - {chapter_title}
          </div>
          <div class="font-semibold">下载《浪客行》第5卷会产生三层文件夹，分别是</div>
          <div class="flex gap-1 text-black">
            <span class="bg-gray-200 rounded-md px-2 w-fit">井上雄彦</span>
            <span class="rounded bg-gray-500 px-1 text-white">/</span>
            <span class="bg-gray-200 rounded-md px-2 w-fit">[井上雄彦] 浪客行(67a18a90cac76a1659ab71f5)</span>
            <span class="rounded bg-gray-500 px-1 text-white">/</span>
            <span class="bg-gray-200 rounded-md px-2 w-fit">5 - 第5卷</span>
          </div>
          <template #trigger>
            <n-input-group class="box-border">
              <n-input-group-label size="small">下载目录格式</n-input-group-label>
              <n-input
                v-model:value="dirFmt"
                size="small"
                @blur="store.config.dirFmt = dirFmt"
                @keydown.enter="store.config.dirFmt = dirFmt" />
            </n-input-group>
          </template>
        </n-tooltip>

        <n-button class="ml-auto mt-2" size="small" @click="showConfigInFileManager">打开配置目录</n-button>
      </div>
    </n-dialog>
  </n-modal>
</template>
