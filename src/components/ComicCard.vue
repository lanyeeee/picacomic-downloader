<script setup lang="ts">
import { commands, ImageRespData } from '../bindings.ts'
import { useStore } from '../store.ts'
import IconButton from './IconButton.vue'
import { PhDownloadSimple, PhFolderOpen } from '@phosphor-icons/vue'

const store = useStore()

const props = defineProps<{
  comicId: string
  comicTitle: string
  comicAuthor: string
  comicCategories: string[]
  comicDownloaded: boolean
  comicDownloadDir: string
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
  const result = await commands.showPathInFileManager(props.comicDownloadDir)
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
            class="font-bold text-lg line-clamp-2 cursor-pointer transition-colors duration-200 hover:text-blue-5"
            @click="pickComic">
            {{ comicTitle }}
          </span>
          <span class="text-red">作者：{{ comicAuthor }}</span>
          <span class="text-gray" v-html="`分类：${comicCategories}`"></span>
        </div>
        <div class="flex">
          <IconButton v-if="comicDownloaded" title="打开下载目录" @click="showComicDownloadDirInFileManager">
            <PhFolderOpen :size="24" />
          </IconButton>
          <IconButton class="ml-auto" title="一键下载所有章节" @click="downloadComic">
            <PhDownloadSimple :size="24" />
          </IconButton>
        </div>
      </div>
    </div>
  </n-card>
</template>
