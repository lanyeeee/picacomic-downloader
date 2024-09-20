<script setup lang="ts">
import {Pagination} from "../bindings.ts";
import {ComicInfo} from "../types.ts";

defineProps<{
  comicInfoPagination: Pagination<ComicInfo> | undefined;
  onClickItem: (comicId: string) => void;
}>();
</script>

<template>
  <div v-if="comicInfoPagination!==undefined" class="flex flex-col gap-row-2 overflow-auto">
    <n-card class="cursor-pointer"
            content-style="padding: 0.25rem;"
            hoverable
            v-for="comicInfo in comicInfoPagination.docs"
            :key="comicInfo._id"
            @click="onClickItem(comicInfo._id)">
      <div class="flex">
        <img class="w-24 object-cover pr-4"
             :src="`${comicInfo.thumb.fileServer}/static/${comicInfo.thumb.path}`"
             alt=""
             referrerpolicy="no-referrer"/>
        <div class="flex flex-col h-full">
          <span class="font-bold text-xl">{{ comicInfo.title }}</span>
          <span class="text-red">作者：{{ comicInfo.author }}</span>
          <span class="text-gray" v-html="`分类：${comicInfo.categories}`"></span>
        </div>
      </div>
    </n-card>
  </div>
</template>