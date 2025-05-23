<script setup lang="ts">
import { path } from '@tauri-apps/api'
import { commands, ImageRespData } from '../bindings.ts'
import { useStore } from '../store.ts'

const store = useStore()

const props = defineProps<{
  comicId: string
  comicTitle: string
  comicAuthor: string
  comicCategories: string[]
  comicDownloaded: boolean
  comicDirName: string
  thumb: ImageRespData
}>()

async function downloadComic() {
  const result = await commands.downloadComic(props.comicId)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
}

async function pickComic() {
  const result = await commands.getComic(props.comicId)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  store.pickedComic = result.data
  store.currentTabName = 'chapter'
}

async function showComicDownloadDirInFileManager() {
  if (store.config === undefined) {
    return
  }
  const comicDir = await path.join(store.config.downloadDir, props.comicDirName)
  const result = await commands.showPathInFileManager(comicDir)
  if (result.status === 'error') {
    console.error(result.error)
  }
}
</script>

<template>
  <n-card content-style="padding: 0.25rem;" hoverable>
    <div class="flex">
      <img
        class="w-24 aspect-[3/4] object-contain mr-4 cursor-pointer transform transition-transform duration-200 hover:scale-106"
        :src="`${thumb.fileServer}/static/${thumb.path}`"
        alt=""
        referrerpolicy="no-referrer"
        @click="pickComic" />
      <div class="flex flex-col w-full justify-between">
        <div class="flex flex-col">
          <span
            class="font-bold text-xl line-clamp-2 cursor-pointer transition-colors duration-200 hover:text-blue-5"
            @click="pickComic">
            {{ comicTitle }}
          </span>
          <span class="text-red">作者：{{ comicAuthor }}</span>
          <span class="text-gray" v-html="`分类：${comicCategories}`"></span>
        </div>
        <div class="flex">
          <n-button v-if="comicDownloaded" size="tiny" @click="showComicDownloadDirInFileManager">
            打开下载目录
          </n-button>
          <n-button size="tiny" class="ml-auto" @click="downloadComic">一键下载所有章节</n-button>
        </div>
      </div>
    </div>
  </n-card>
</template>
