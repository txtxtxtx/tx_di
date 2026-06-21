<template>
  <el-dialog
    v-model="dialogVisible"
    :title="fileName || '文件预览'"
    :width="dialogWidth"
    class="file-preview-dialog"
    destroy-on-close
    @close="handleClose"
  >
    <div class="preview-container" v-loading="loading" element-loading-text="加载中...">
      <!-- Images -->
      <div v-if="fileCategory === 'image'" class="preview-image">
        <img :src="mediaUrl" :alt="fileName" @load="loading = false" @error="handleLoadError" />
      </div>

      <!-- PDF：用 pdfjs-dist 直接渲染到 canvas -->
      <div v-else-if="fileCategory === 'pdf'" class="preview-pdf">
        <div v-if="pdfLoading" class="pdf-loading">
          <el-icon class="is-loading" :size="32"><Loading /></el-icon>
          <span>PDF 加载中...</span>
        </div>
        <template v-else>
          <canvas v-for="p in pdfPages" :key="p" :ref="canvasRef" />
        </template>
      </div>

      <!-- Word -->
      <div v-else-if="fileCategory === 'word'" class="preview-office">
        <VueOfficeDocx
          :src="officeSrc"
          @rendered="loading = false"
          @error="handleOfficeError"
        />
      </div>

      <!-- Excel -->
      <div v-else-if="fileCategory === 'excel'" class="preview-office">
        <VueOfficeExcel
          :src="officeSrc"
          @rendered="loading = false"
          @error="handleOfficeError"
        />
      </div>

      <!-- PowerPoint -->
      <div v-else-if="fileCategory === 'ppt'" class="preview-office">
        <VueOfficePptx
          :src="officeSrc"
          @rendered="loading = false"
          @error="handleOfficeError"
        />
      </div>

      <!-- Video -->
      <div v-else-if="fileCategory === 'video'" class="preview-video">
        <video :src="mediaUrl" controls @loadeddata="loading = false" @error="handleLoadError">
          您的浏览器不支持视频播放
        </video>
      </div>

      <!-- Audio -->
      <div v-else-if="fileCategory === 'audio'" class="preview-audio">
        <audio :src="mediaUrl" controls @canplay="loading = false" @error="handleLoadError">
          您的浏览器不支持音频播放
        </audio>
      </div>

      <!-- Text/Code -->
      <div v-else-if="fileCategory === 'text'" class="preview-text">
        <pre v-if="textContent">{{ textContent }}</pre>
      </div>

      <!-- Unsupported -->
      <div v-else class="preview-unsupported">
        <el-empty description="不支持预览此文件类型">
          <el-button type="primary" @click="downloadFile">下载文件</el-button>
        </el-empty>
      </div>
    </div>
  </el-dialog>
</template>

<script setup>
import { ref, computed, watch, nextTick, onBeforeUnmount } from 'vue'
import axios from 'axios'
import * as pdfjsLib from 'pdfjs-dist'
import VueOfficeDocx from '@vue-office/docx'
import '@vue-office/docx/lib/index.css'
import VueOfficeExcel from '@vue-office/excel'
import '@vue-office/excel/lib/index.css'
import VueOfficePptx from '@vue-office/pptx'
import { getPreviewUrl } from '@/api/file'
import { Loading } from '@element-plus/icons-vue'

// 配置 PDF.js Worker
pdfjsLib.GlobalWorkerOptions.workerSrc = '/pdf.worker.min.mjs'

const props = defineProps({
  visible: {
    type: Boolean,
    default: false
  },
  fileId: {
    type: [String, Number],
    default: null
  },
  fileName: {
    type: String,
    default: ''
  },
  fileType: {
    type: String,
    default: null
  }
})

const emit = defineEmits(['update:visible'])

const loading = ref(false)
const textContent = ref('')
const officeSrc = ref(null)
const mediaUrl = ref('')

// --- PDF 状态 ---
const pdfSrc = ref('')
const pdfPages = ref(0)
const pdfLoading = ref(false)
let pdfDoc = null

// --- canvas ref 回调 ---
const canvasRefs = []
function canvasRef(el) {
  if (el) canvasRefs.push(el)
}

// --- Dialog v-model ---
const dialogVisible = computed({
  get: () => props.visible,
  set: (val) => emit('update:visible', val)
})

// --- File extension helpers ---
const fileExtension = computed(() => {
  if (!props.fileName) return ''
  const parts = props.fileName.split('.')
  return parts.length > 1 ? parts.pop().toLowerCase() : ''
})

const IMAGE_EXTS = ['jpg', 'jpeg', 'png', 'gif', 'svg', 'webp', 'bmp', 'avif', 'ico']
const VIDEO_EXTS = ['mp4', 'webm', 'ogg', 'mov', 'avi']
const AUDIO_EXTS = ['mp3', 'wav', 'ogg', 'aac', 'flac', 'm4a']
const TEXT_EXTS = [
  'txt', 'log', 'md', 'csv', 'json', 'xml', 'yml', 'yaml', 'toml', 'ini',
  'html', 'css', 'js', 'ts', 'vue', 'rs', 'py', 'java', 'go', 'c', 'cpp',
  'sh', 'bat', 'sql'
]

const fileCategory = computed(() => {
  const ext = fileExtension.value
  if (!ext) return 'unknown'
  if (IMAGE_EXTS.includes(ext)) return 'image'
  if (ext === 'pdf') return 'pdf'
  if (ext === 'docx') return 'word'
  if (ext === 'xlsx' || ext === 'xls') return 'excel'
  if (ext === 'pptx' || ext === 'ppt') return 'ppt'
  if (VIDEO_EXTS.includes(ext)) return 'video'
  if (AUDIO_EXTS.includes(ext)) return 'audio'
  if (TEXT_EXTS.includes(ext)) return 'text'
  return 'unknown'
})

const dialogWidth = computed(() => {
  switch (fileCategory.value) {
    case 'image': return '70vw'
    case 'pdf': return '80vw'
    case 'video': return '70vw'
    case 'audio': return '500px'
    case 'text': return '70vw'
    case 'excel': return '90vw'
    case 'word': return '80vw'
    case 'ppt': return '90vw'
    default: return '500px'
  }
})

// --- Auth helper ---
function getAuthHeaders() {
  const token = localStorage.getItem('token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}

// --- 渲染 PDF 所有页面到 canvas ---
async function renderAllPdfPages() {
  if (!pdfDoc) return
  canvasRefs.length = 0
  await nextTick()
  await nextTick()
  const SCALE = 1.5
  for (let i = 1; i <= pdfDoc.numPages; i++) {
    const canvas = canvasRefs[i - 1]
    if (!canvas) continue
    const page = await pdfDoc.getPage(i)
    const viewport = page.getViewport({ scale: SCALE })
    canvas.width = viewport.width
    canvas.height = viewport.height
    await page.render({ canvasContext: canvas.getContext('2d'), viewport }).promise
  }
}

// --- 加载 PDF 文档 ---
async function loadPdf(url) {
  if (pdfDoc) {
    pdfDoc.destroy()
    pdfDoc = null
  }
  pdfLoading.value = true
  pdfPages.value = 0
  try {
    pdfDoc = await pdfjsLib.getDocument(url).promise
    pdfPages.value = pdfDoc.numPages
    await renderAllPdfPages()
  } catch (err) {
    console.error('PDF 加载失败:', err)
  } finally {
    pdfLoading.value = false
    loading.value = false
  }
}

// --- Load content when dialog opens ---
watch(
  () => props.visible,
  async (val) => {
    if (!val || !props.fileId) return

    loading.value = true
    textContent.value = ''
    officeSrc.value = null
    pdfSrc.value = ''
    pdfPages.value = 0
    mediaUrl.value = ''

    const category = fileCategory.value

    try {
      const previewRes = await getPreviewUrl(String(props.fileId))
      const url = previewRes.data.url

      if (category === 'image' || category === 'video' || category === 'audio') {
        mediaUrl.value = url
        loading.value = false
      } else if (category === 'pdf') {
        pdfSrc.value = url
        await loadPdf(url)
      } else if (category === 'word' || category === 'excel' || category === 'ppt') {
        const resp = await axios.get(url, {
          responseType: 'arraybuffer',
          headers: getAuthHeaders()
        })
        officeSrc.value = resp.data
        loading.value = false
      } else if (category === 'text') {
        const resp = await axios.get(url, {
          responseType: 'text',
          headers: getAuthHeaders()
        })
        textContent.value = typeof resp.data === 'string' ? resp.data : JSON.stringify(resp.data, null, 2)
        loading.value = false
      } else {
        loading.value = false
      }
    } catch (err) {
      console.error('文件加载失败:', err)
      loading.value = false
    }
  }
)

// --- Error handlers ---
function handleLoadError() {
  loading.value = false
  console.error('资源加载失败')
}

function handleOfficeError(err) {
  loading.value = false
  console.error('Office 文件渲染失败:', err)
}

// --- Cleanup ---
function handleClose() {
  if (pdfDoc) {
    pdfDoc.destroy()
    pdfDoc = null
  }
  pdfSrc.value = ''
  pdfPages.value = 0
  mediaUrl.value = ''
  textContent.value = ''
  officeSrc.value = null
  loading.value = false
}

onBeforeUnmount(() => {
  if (pdfDoc) {
    pdfDoc.destroy()
    pdfDoc = null
  }
})

// --- Download fallback ---
function downloadFile() {
  if (!props.fileId) return
  const a = document.createElement('a')
  a.href = `/api/file/${props.fileId}/download`
  a.download = props.fileName || ''
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
}
</script>

<style>
/* 非 scoped：控制 el-dialog 整体高度，防止撑出视口导致父页面滚动 */
.file-preview-dialog {
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  margin: 5vh auto !important;
}
.file-preview-dialog .el-dialog__header {
  flex-shrink: 0;
}
.file-preview-dialog .el-dialog__body {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}
</style>

<style scoped>
.preview-container {
  min-height: 200px;
  display: flex;
  align-items: flex-start;
  justify-content: center;
}

.preview-image img {
  max-width: 100%;
  max-height: 80vh;
  object-fit: contain;
}

.preview-pdf {
  width: 100%;
}

.preview-pdf canvas {
  display: block;
  max-width: 100%;
  height: auto;
  margin-bottom: 8px;
}

.preview-pdf .pdf-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 40px;
  color: var(--el-text-color-secondary);
}

.preview-office {
  width: 100%;
}

.preview-video video {
  max-width: 100%;
  max-height: 75vh;
}

.preview-audio {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px 0;
}

.preview-audio audio {
  width: 80%;
}

.preview-text {
  width: 100%;
}

.preview-text pre {
  margin: 0;
  padding: 16px;
  font-family: 'Cascadia Code', 'Fira Code', 'Consolas', 'Monaco', monospace;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-wrap: break-word;
  background: #f5f7fa;
  border-radius: 4px;
}

.preview-unsupported {
  padding: 40px 0;
}
</style>
