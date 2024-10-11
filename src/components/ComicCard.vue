<script setup lang="ts">
import {ComicInfo} from "../types.ts";

defineProps<{
  comicInfo: ComicInfo;
  onClickItem: (comicId: string) => void;
}>();
</script>

<template>
  <n-card content-style="padding: 0.25rem;" hoverable>
    <div class="flex">
      <img
          class="w-24 aspect-[3/4] object-contain mr-4 cursor-pointer transform transition-transform duration-200 hover:scale-106"
          :src="`${comicInfo.thumb.fileServer}/static/${comicInfo.thumb.path}`"
          alt=""
          referrerpolicy="no-referrer"
          @click="onClickItem(comicInfo._id)"/>
      <div class="flex flex-col w-full">
        <span class="font-bold text-xl line-clamp-2 cursor-pointer hover:text-blue-5"
              @click="onClickItem(comicInfo._id)">{{ comicInfo.title }}</span>
        <span class="text-red">作者：{{ comicInfo.author }}</span>
        <span class="text-gray" v-html="`分类：${comicInfo.categories}`"></span>
      </div>
    </div>
  </n-card>
</template>