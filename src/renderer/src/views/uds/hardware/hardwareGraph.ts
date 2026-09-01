// 总线驱动(root 椭圆) -> fork 分叉点 -> 通道(直角折线) -> 干线(trunk) -> 子节点
// (IA/Node,竖直支线挂在干线上方)。所有连线均为我们自己算好坐标后用 vertices 画出的
// 直角折线,不依赖路由算法的黑盒行为。
import * as joint from '@joint/core'
import { EventEmitter } from 'events'

export type ChildKind = 'ia' | 'node'

export interface ChildVM {
  id: string
  kind: ChildKind
  name: string
}

export interface ChannelSummary {
  vendorDevice: string // 例如 "ZLG · USBCAN-II"
  baudRes: string // 例如 "500 kbit/s · 120Ω"
}

export interface ChannelVM {
  id: string
  name: string
  configured: boolean
  summary: ChannelSummary | null
  canMoveUp: boolean
  canMoveDown: boolean
  /** Already accounts for bus-type support, device configuration, and the one-IA-per-channel limit. */
  canAddIa: boolean
  children: ChildVM[]
}

// 布局常量:通道行高是固定值,子节点沿干线横向扩展,不会撑高这一行。
const ROOT_W = 84
const ROOT_H = 52
const CH_X = 220
const CH_W = 126
const CH_H = 44
const CD_W = 64
const CD_H = 28
const STUB_H = 26
const CHILD_GAP_X = 78
const TRUNK_PAD = 40
const ROW_H = 140

const LINE_ATTRS = {
  stroke: 'var(--el-border-color)',
  strokeWidth: 1.5,
  targetMarker: { type: 'none' }
}

const IA_LIMIT_TOOLTIP = '每条通道仅限一个接口'

// @joint/core 要求自定义元素通过 define() 注册固定 markup(与 udsView.ts 同一套已验证写法),
// 直接 new joint.dia.Element({ markup }) 在本版本里不渲染视图。可选节点(上下箭头等)始终存在,
// 用 display 属性切换显隐。
const RootElement = joint.dia.Element.define(
  'hw.Root',
  {},
  {
    markup: [
      { tagName: 'rect', selector: 'body' },
      { tagName: 'text', selector: 'title' },
      { tagName: 'text', selector: 'subtitle' }
    ]
  }
)

const ForkElement = joint.dia.Element.define(
  'hw.Fork',
  {},
  { markup: [{ tagName: 'circle', selector: 'body' }] }
)

const ButtonElement = joint.dia.Element.define(
  'hw.Button',
  {},
  {
    markup: [
      { tagName: 'circle', selector: 'body' },
      { tagName: 'text', selector: 'glyph' }
    ]
  }
)

const ChannelElement = joint.dia.Element.define(
  'hw.Channel',
  {},
  {
    markup: [
      { tagName: 'rect', selector: 'body' },
      { tagName: 'text', selector: 'title' },
      { tagName: 'text', selector: 'line2' },
      { tagName: 'text', selector: 'line3' },
      { tagName: 'circle', selector: 'gearBody' },
      { tagName: 'text', selector: 'gearGlyph' },
      { tagName: 'circle', selector: 'delBody' },
      { tagName: 'text', selector: 'delGlyph' },
      {
        tagName: 'circle',
        selector: 'iaBody',
        children: [{ tagName: 'title', selector: 'iaTitle' }]
      },
      { tagName: 'text', selector: 'iaGlyph' },
      { tagName: 'text', selector: 'upBtn' },
      { tagName: 'text', selector: 'downBtn' }
    ]
  }
)

const ChildElement = joint.dia.Element.define(
  'hw.Child',
  {},
  {
    markup: [
      { tagName: 'rect', selector: 'body' },
      { tagName: 'text', selector: 'glyph' },
      { tagName: 'text', selector: 'label' },
      { tagName: 'circle', selector: 'delBody' },
      { tagName: 'text', selector: 'delGlyph' }
    ]
  }
)

export type HardwareBoardEvent =
  | 'addChannel'
  | 'gear'
  | 'moveUp'
  | 'moveDown'
  | 'addIa'
  | 'addNode'
  | 'removeChannel'
  | 'removeChild'
  | 'editChild'

type BoardListener = (id?: string) => void

/**
 * Renders the "bus root -> fork -> channels -> trunk -> IA/Node children" board for a
 * single bus type using @joint/core. Every connection is an explicit right-angle
 * polyline (vertices computed by us), not an auto-router, so layout stays deterministic.
 */
export class HardwareBoard {
  private graph: joint.dia.Graph
  private paper: joint.dia.Paper | null = null
  private listeners: Partial<Record<HardwareBoardEvent, BoardListener[]>> = {}
  private childKindMap = new Map<string, ChildKind>()

  constructor(graph: joint.dia.Graph) {
    this.graph = graph
  }

  setPaper(paper: joint.dia.Paper) {
    this.paper = paper
    // 用事件委托识别点击的是哪个按钮:markup 里给每个可点击子元素加 data-action/data-id,
    // 不依赖 @joint/core 开源版没有的 Halo 之类的商业插件。
    paper.on('element:pointerclick', (elementView: joint.dia.ElementView, evt: unknown) => {
      const nativeEvt = (evt as { target?: SVGElement })?.target
      const el = (nativeEvt?.closest?.('[data-action]') ?? nativeEvt) as SVGElement | null
      const action = el?.getAttribute?.('data-action')
      if (!action) return
      const targetId =
        el?.getAttribute?.('data-id') || (elementView.model.get('hwId') as string | undefined)
      this.dispatch(action as HardwareBoardEvent, targetId)
    })

    // 删除按钮默认 display:none,hover 整个节点时临时显示(SVG 内联属性,不是靠 CSS class)。
    paper.on('element:mouseenter', (elementView: joint.dia.ElementView) => {
      const model = elementView.model
      if (model.attr('delBody')) {
        model.attr('delBody/display', 'block')
        model.attr('delGlyph/display', 'block')
      }
    })
    paper.on('element:mouseleave', (elementView: joint.dia.ElementView) => {
      const model = elementView.model
      if (model.attr('delBody')) {
        model.attr('delBody/display', 'none')
        model.attr('delGlyph/display', 'none')
      }
    })
  }

  on(event: HardwareBoardEvent, cb: BoardListener) {
    ;(this.listeners[event] ||= []).push(cb)
  }

  private dispatch(event: HardwareBoardEvent, id?: string) {
    for (const cb of this.listeners[event] || []) cb(id)
  }

  getChildKind(id: string): ChildKind | undefined {
    return this.childKindMap.get(id)
  }

  /**
   * @param viewportW/viewportH lower bound for the paper size (the visible container), so
   * short content still fills the viewport while long content grows and scrolls instead
   * of being squeezed.
   */
  render(busLabel: string, channels: ChannelVM[], viewportW = 0, viewportH = 0) {
    this.graph.clear()
    this.childKindMap.clear()

    const rowsTotalH = Math.max(channels.length, 1) * ROW_H + 20
    const maxChildren = channels.reduce((m, c) => Math.max(m, c.children.length), 0)
    const canvasW = Math.max(660, CH_X + CH_W + TRUNK_PAD + maxChildren * CHILD_GAP_X + 80)

    const rootX = 16
    const rootY = rowsTotalH / 2 - ROOT_H / 2 + 10
    this.graph.addCell(this.makeRoot(rootX, rootY, busLabel))

    const forkX = rootX + ROOT_W + 26
    const forkY = rootY + ROOT_H / 2
    this.graph.addCell(this.makeFork(forkX, forkY))
    this.graph.addCell(this.makeLine([rootX + ROOT_W, forkY], [forkX, forkY]))

    // "+" 添加通道按钮,挂在 fork 旁边(整条总线只有一个,不属于任何具体通道)。
    this.graph.addCell(this.makeAddChannelButton(forkX + 6, forkY - 26))

    channels.forEach((ch, i) => {
      const rowTop = i * ROW_H + 10
      const trunkY = rowTop + ROW_H - 30
      const chY = trunkY - CH_H / 2

      // fork -> 通道:一个 vertex 就能拐出直角(先竖直到 trunkY,再水平到 CH_X)。
      this.graph.addCell(this.makeLine([forkX, forkY], [CH_X, trunkY], [{ x: forkX, y: trunkY }]))

      this.graph.addCell(this.makeChannel(CH_X, chY, ch))

      const trunkEndX = CH_X + CH_W + TRUNK_PAD + ch.children.length * CHILD_GAP_X
      // 干线本身:通道右边缘水平延伸出去的一条线。
      this.graph.addCell(this.makeLine([CH_X + CH_W, trunkY], [trunkEndX, trunkY]))

      ch.children.forEach((cd, j) => {
        const cx = CH_X + CH_W + TRUNK_PAD + j * CHILD_GAP_X
        const cy = trunkY - STUB_H - CD_H
        this.childKindMap.set(cd.id, cd.kind)
        this.graph.addCell(this.makeChild(cx, cy, cd))
        // 支线:竖直短线,从干线连到子节点方块底边。
        this.graph.addCell(
          this.makeLine([cx + CD_W / 2, trunkY], [cx + CD_W / 2, trunkY - STUB_H])
        )
      })

      // "添加脚本节点" 按钮固定挂在干线最右端。
      this.graph.addCell(this.makeAddNodeButton(trunkEndX + 6, trunkY - 8, ch.id))
    })

    this.paper?.setDimensions(Math.max(canvasW, viewportW), Math.max(rowsTotalH + 20, viewportH))
  }

  // ---------- 各类节点的构建 ----------

  private makeRoot(x: number, y: number, label: string) {
    return new RootElement({
      position: { x, y },
      size: { width: ROOT_W, height: ROOT_H },
      attrs: {
        body: {
          width: ROOT_W,
          height: ROOT_H,
          rx: 7,
          ry: 7,
          fill: 'var(--el-fill-color-blank)',
          stroke: 'var(--el-border-color-darker)',
          strokeWidth: 1
        },
        title: {
          text: label,
          x: ROOT_W / 2,
          y: ROOT_H / 2 - 4,
          textAnchor: 'middle',
          fontSize: 13,
          fontWeight: 600,
          fill: 'var(--el-text-color-primary)'
        },
        subtitle: {
          text: '总线驱动',
          x: ROOT_W / 2,
          y: ROOT_H / 2 + 12,
          textAnchor: 'middle',
          fontSize: 10,
          fill: 'var(--el-text-color-secondary)'
        }
      }
    })
  }

  private makeFork(x: number, y: number) {
    return new ForkElement({
      position: { x: x - 3, y: y - 3 },
      size: { width: 6, height: 6 },
      attrs: {
        body: {
          cx: 3,
          cy: 3,
          r: 3,
          fill: 'var(--el-text-color-secondary)'
        }
      }
    })
  }

  private makeAddChannelButton(x: number, y: number) {
    return this.makeRoundButton(x, y, '+', 'addChannel')
  }

  private makeAddNodeButton(x: number, y: number, channelId: string) {
    return this.makeRoundButton(x, y, '</>', 'addNode', channelId)
  }

  private makeRoundButton(x: number, y: number, glyph: string, action: string, id?: string) {
    const size = 17
    return new ButtonElement({
      position: { x, y },
      size: { width: size, height: size },
      hwId: id,
      attrs: {
        body: {
          cx: size / 2,
          cy: size / 2,
          r: size / 2 - 0.5,
          fill: 'var(--el-fill-color-blank)',
          stroke: 'var(--el-color-primary)',
          strokeWidth: 1,
          cursor: 'pointer',
          'data-action': action,
          ...(id ? { 'data-id': id } : {})
        },
        glyph: {
          text: glyph,
          x: size / 2,
          y: size / 2 + 3,
          textAnchor: 'middle',
          fontSize: 9,
          fill: 'var(--el-color-primary)',
          pointerEvents: 'none' // 避免点在文字上时 closest() 找不到父级 data-action
        }
      }
    })
  }

  private makeChannel(x: number, y: number, ch: ChannelVM) {
    const line2 = ch.configured && ch.summary ? ch.summary.vendorDevice : '未映射硬件'
    const line3 = ch.configured && ch.summary ? ch.summary.baudRes : ''
    const line2Color = ch.configured
      ? 'var(--el-color-primary)'
      : 'var(--el-text-color-placeholder)'
    const hasIa = ch.children.some((c) => c.kind === 'ia')

    const attrs: joint.dia.Cell.Selectors = {
      body: {
        width: CH_W,
        height: CH_H,
        rx: 7,
        ry: 7,
        fill: 'var(--el-color-primary-light-9)',
        stroke: 'var(--el-color-primary-light-5)',
        strokeWidth: 1
      },
      title: {
        text: ch.name,
        x: CH_W / 2,
        y: 15,
        textAnchor: 'middle',
        fontSize: 12,
        fontWeight: 600,
        fill: 'var(--el-color-primary)'
      },
      line2: {
        text: line2,
        x: CH_W / 2,
        y: 27,
        textAnchor: 'middle',
        fontSize: 9,
        fill: line2Color
      },
      line3: {
        text: line3,
        x: CH_W / 2,
        y: 38,
        textAnchor: 'middle',
        fontSize: 9,
        fill: 'var(--el-text-color-secondary)'
      },
      gearBody: {
        cx: -8,
        cy: -8,
        r: 8,
        fill: 'var(--el-fill-color-blank)',
        stroke: 'var(--el-color-primary)',
        strokeWidth: 1,
        cursor: 'pointer',
        'data-action': 'gear',
        'data-id': ch.id
      },
      gearGlyph: {
        text: '\u2699',
        x: -8,
        y: -5,
        textAnchor: 'middle',
        fontSize: 10,
        fill: 'var(--el-color-primary)',
        pointerEvents: 'none'
      },
      delBody: {
        cx: CH_W + 6,
        cy: -6,
        r: 7,
        fill: 'var(--el-color-danger-light-9)',
        stroke: 'var(--el-color-danger)',
        strokeWidth: 1,
        cursor: 'pointer',
        'data-action': 'removeChannel',
        'data-id': ch.id,
        display: 'none'
      },
      delGlyph: {
        text: '\u00d7',
        x: CH_W + 6,
        y: -3,
        textAnchor: 'middle',
        fontSize: 10,
        fill: 'var(--el-color-danger)',
        pointerEvents: 'none',
        display: 'none'
      },
      iaBody: {
        cx: CH_W + 8,
        cy: CH_H / 2,
        r: 8,
        fill: ch.canAddIa ? 'var(--el-fill-color-blank)' : 'var(--el-fill-color)',
        stroke: 'var(--el-color-primary)',
        strokeWidth: 1,
        opacity: ch.canAddIa ? 1 : 0.35,
        cursor: ch.canAddIa ? 'pointer' : 'default',
        ...(ch.canAddIa
          ? { 'data-action': 'addIa', 'data-id': ch.id }
          : { 'data-action': null, 'data-id': null })
      },
      iaTitle: {
        text: !ch.canAddIa && hasIa ? IA_LIMIT_TOOLTIP : ''
      },
      iaGlyph: {
        text: '\ud83d\udd11',
        x: CH_W + 8,
        y: CH_H / 2 + 3,
        textAnchor: 'middle',
        fontSize: 8,
        pointerEvents: 'none'
      },
      upBtn: {
        text: '\u25b2',
        x: -20,
        y: 10,
        fontSize: 9,
        fill: 'var(--el-text-color-secondary)',
        cursor: 'pointer',
        display: ch.canMoveUp ? 'block' : 'none',
        'data-action': 'moveUp',
        'data-id': ch.id
      },
      downBtn: {
        text: '\u25bc',
        x: -20,
        y: CH_H - 4,
        fontSize: 9,
        fill: 'var(--el-text-color-secondary)',
        cursor: 'pointer',
        display: ch.canMoveDown ? 'block' : 'none',
        'data-action': 'moveDown',
        'data-id': ch.id
      }
    }

    return new ChannelElement({
      position: { x, y },
      size: { width: CH_W, height: CH_H },
      hwId: ch.id,
      attrs
    })
  }

  private makeChild(x: number, y: number, cd: ChildVM) {
    const isIa = cd.kind === 'ia'
    return new ChildElement({
      position: { x, y },
      size: { width: CD_W, height: CD_H },
      hwId: cd.id,
      attrs: {
        body: {
          width: CD_W,
          height: CD_H,
          rx: 6,
          ry: 6,
          fill: 'var(--el-fill-color-blank)',
          stroke: isIa ? 'var(--el-border-color)' : 'var(--el-color-primary-light-5)',
          strokeWidth: 1,
          cursor: 'pointer',
          'data-action': 'editChild',
          'data-id': cd.id
        },
        glyph: {
          text: isIa ? '\ud83d\udd11' : '</>',
          x: 12,
          y: CD_H / 2 + 3,
          fontSize: 9,
          pointerEvents: 'none',
          fill: isIa ? 'var(--el-text-color-secondary)' : 'var(--el-color-primary)'
        },
        label: {
          text: cd.name,
          x: 24,
          y: CD_H / 2 + 3,
          fontSize: 9,
          pointerEvents: 'none',
          fill: isIa ? 'var(--el-text-color-secondary)' : 'var(--el-color-primary)'
        },
        delBody: {
          cx: CD_W - 2,
          cy: -2,
          r: 6,
          fill: 'var(--el-color-danger-light-9)',
          stroke: 'var(--el-color-danger)',
          strokeWidth: 1,
          cursor: 'pointer',
          'data-action': 'removeChild',
          'data-id': cd.id,
          display: 'none'
        },
        delGlyph: {
          text: '\u00d7',
          x: CD_W - 2,
          y: 1,
          textAnchor: 'middle',
          fontSize: 8,
          fill: 'var(--el-color-danger)',
          pointerEvents: 'none',
          display: 'none'
        }
      }
    })
  }

  private makeLine(
    p1: [number, number],
    p2: [number, number],
    vertices: { x: number; y: number }[] = []
  ) {
    return new joint.shapes.standard.Link({
      source: { x: p1[0], y: p1[1] },
      target: { x: p2[0], y: p2[1] },
      vertices,
      connector: { name: 'normal' },
      attrs: { line: LINE_ATTRS },
      interactive: false, // 装饰性连线,不允许用户拖拽改路径
      z: -1
    })
  }
}

