<script setup lang="ts">
import { commands } from '../bindings.ts'
import { path } from '@tauri-apps/api'
import { appDataDir } from '@tauri-apps/api/path'
import { useStore } from '../store.ts'
import { ref } from 'vue'

const store = useStore()

const showing = defineModel<boolean>('showing', { required: true })

const comicDirNameFmt = ref<string>(store.config?.comicDirNameFmt ?? '')
const chapterDirNameFmt = ref<string>(store.config?.chapterDirNameFmt ?? '')

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
    <n-dialog :showIcon="false" title="配置" @close="showing = false">
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
            <div>保持原图格式，不做任何转换</div>
            <template #trigger>
              <n-radio value="Original">原始格式</n-radio>
            </template>
          </n-tooltip>
        </n-radio-group>

        <n-tooltip placement="top" trigger="hover">
          <div class="font-semibold">
            <span class="text-pink">漫画名</span>
            <span>可用字段：</span>
          </div>
          <div>
            <div>
              <span class="rounded bg-gray-500 px-1">comic_id</span>
              <span class="ml-2">漫画ID</span>
            </div>
            <div>
              <span class="rounded bg-gray-500 px-1">comic_title</span>
              <span class="ml-2">漫画标题</span>
            </div>
            <div>
              <span class="rounded bg-gray-500 px-1">author</span>
              <span class="ml-2">作者</span>
            </div>
          </div>
          <div class="font-semibold mt-2">例如格式</div>
          <div class="bg-gray-200 rounded-md p-1 text-black">[{author}] {comic_title}({comic_id})</div>
          <div class="font-semibold">下载《电锯人》的结果：</div>
          <div class="bg-gray-200 rounded-md p-1 text-black">
            [藤本树（藤本タツキ）] 电锯人(5f606646d50a7c0733961549)
          </div>
          <template #trigger>
            <n-input-group class="box-border">
              <n-input-group-label size="small">漫画名格式</n-input-group-label>
              <n-input
                v-model:value="comicDirNameFmt"
                size="small"
                @blur="store.config.comicDirNameFmt = comicDirNameFmt"
                @keydown.enter="store.config.comicDirNameFmt = comicDirNameFmt" />
            </n-input-group>
          </template>
        </n-tooltip>

        <n-tooltip placement="top" trigger="hover">
          <div class="font-semibold">
            <span class="text-pink">章节名</span>
            <span>可用字段：</span>
          </div>
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
          <div class="bg-gray-200 rounded-md p-1 text-black">
            [{author}] {comic_title}({comic_id}) - {order} - {chapter_title}({chapter_id})
          </div>
          <div class="font-semibold">下载《电锯人》第20话的结果：</div>
          <div class="bg-gray-200 rounded-md p-1 text-black">
            [藤本树（藤本タツキ）] 电锯人(5f606646d50a7c0733961549) - 20 - 第20話(5f623435a183ac0739ccf041)
          </div>
          <template #trigger>
            <n-input-group class="box-border">
              <n-input-group-label size="small">章节名格式</n-input-group-label>
              <n-input
                v-model:value="chapterDirNameFmt"
                size="small"
                @blur="store.config.chapterDirNameFmt = chapterDirNameFmt"
                @keydown.enter="store.config.chapterDirNameFmt = chapterDirNameFmt" />
            </n-input-group>
          </template>
        </n-tooltip>

        <n-button class="ml-auto mt-2" size="small" @click="showConfigInFileManager">打开配置目录</n-button>
      </div>
    </n-dialog>
  </n-modal>
</template>
