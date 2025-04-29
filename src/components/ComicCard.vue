<script setup lang="ts">
import { ComicInfo } from '../types.ts'
import { commands } from '../bindings.ts'
import { useStore } from '../store.ts'

const store = useStore()

const props = defineProps<{
  comicInfo: ComicInfo
}>()

async function downloadComic() {
  const result = await commands.downloadComic(props.comicInfo._id)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
}

async function pickComic() {
  const result = await commands.getComic(props.comicInfo._id)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  store.pickedComic = result.data
  store.currentTabName = 'chapter'
}
</script>

<template>
  <n-card content-style="padding: 0.25rem;" hoverable>
    <div class="flex">
      <img
        class="w-24 aspect-[3/4] object-contain mr-4 cursor-pointer transform transition-transform duration-200 hover:scale-106"
        :src="`${comicInfo.thumb.fileServer}/static/${comicInfo.thumb.path}`"
        alt=""
        referrerpolicy="no-referrer"
        @click="pickComic" />
      <div class="flex flex-col w-full justify-between">
        <div class="flex flex-col">
          <span
            class="font-bold text-xl line-clamp-2 cursor-pointer transition-colors duration-200 hover:text-blue-5"
            @click="pickComic">
            {{ comicInfo.title }}
          </span>
          <span class="text-red">作者：{{ comicInfo.author }}</span>
          <span class="text-gray" v-html="`分类：${comicInfo.categories}`"></span>
        </div>
        <n-button size="tiny" class="ml-auto" @click="downloadComic">一键下载所有章节</n-button>
      </div>
    </div>
  </n-card>
</template>
