<script setup lang="ts">
import { Comic, commands } from '../bindings.ts'
import { useStore } from '../store.ts'

const props = defineProps<{
  comic: Comic
}>()

const store = useStore()

function pickComic() {
  store.pickedComic = props.comic
  store.currentTabName = 'chapter'
}

// 导出cbz
async function exportCbz() {
  const result = await commands.exportCbz(props.comic)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
}

async function exportPdf() {
  const result = await commands.exportPdf(props.comic)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
}

async function showComicDownloadDirInFileManager() {
  if (store.config === undefined) {
    return
  }

  const comicDownloadDir = props.comic.comicDownloadDir

  if (comicDownloadDir === undefined || comicDownloadDir === null) {
    console.error('comicDownloadDir的值为undefined或null')
    return
  }

  const result = await commands.showPathInFileManager(comicDownloadDir)
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
        :src="`${comic.thumb.fileServer}/static/${comic.thumb.path}`"
        alt=""
        referrerpolicy="no-referrer"
        @click="pickComic" />
      <div class="flex flex-col w-full">
        <span
          class="font-bold text-xl line-clamp-2 cursor-pointer transition-colors duration-200 hover:text-blue-5"
          @click="pickComic">
          {{ comic.title }}
        </span>
        <span class="text-red">作者：{{ comic.author }}</span>
        <span class="text-gray" v-html="`分类：${comic.categories}`"></span>
        <div class="flex mt-auto gap-col-2">
          <n-button size="tiny" @click="showComicDownloadDirInFileManager">打开下载目录</n-button>
          <n-button class="ml-auto" size="tiny" @click="exportCbz">导出cbz</n-button>
          <n-button size="tiny" @click="exportPdf">导出pdf</n-button>
        </div>
      </div>
    </div>
  </n-card>
</template>
