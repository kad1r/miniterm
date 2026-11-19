export interface Folder {
  id: string;
  kind: "folder";
  name: string;
  expanded: boolean;
  children: Node[];
}

export interface Workspace {
  id: string;
  kind: "workspace";
  name: string;
  path: string;
  /** The workspace default: what a newly opened pane gets, and the fallback for
   *  any cell `paneTools` does not cover. */
  aiToolId: string | null;
  shellId: string | null;
  rows: number;
  cols: number;
  rowSizes: number[];
  colSizes: number[];
  /** Per-cell AI tool, in cell order — one workspace can run Claude in two panes,
   *  Gemini in a third and a bare shell in a fourth. Read it through `toolsFor()`
   *  in panes.ts rather than directly: a config written before this field existed
   *  carries an empty list, and the list has to track the grid's size. */
  paneTools: (string | null)[];
  /** Per-cell agent name, in cell order. Optional because an older config has
   *  none; read it through `namesFor()` in agents.ts, which fills the gaps. */
  paneNames?: (string | null)[];
  /** The workspace's task board. Absent until the first card or column change;
   *  read it through `boardOf()` in board.ts, which supplies the default columns. */
  board?: Board;
}

export interface BoardCard {
  id: string;
  title: string;
  note: string;
  /** Free-text labels; a tag's colour is derived from its name. */
  tags: string[];
  /** Calendar day as `YYYY-MM-DD`, with no time or zone attached. */
  due: string | null;
}

export interface BoardColumn {
  id: string;
  name: string;
  /** A key from `PALETTE` in board.ts. Absent on columns that never had theirs
   *  changed; those take one by position. */
  color?: string;
  /** In display order, top to bottom. */
  cards: BoardCard[];
}

export interface Board {
  /** In display order, left to right. Never empty once stored. */
  columns: BoardColumn[];
}

export type Node = Folder | Workspace;

export interface AiTool {
  id: string;
  name: string;
  command: string;
}

export interface DirAlias {
  id: string;
  alias: string;
  path: string;
}

export interface Config {
  version: number;
  aiTools: AiTool[];
  directories: DirAlias[];
  recentDirs: string[];
  defaultShellId: string;
  tree: Node[];
}

export interface ShellInfo {
  id: string;
  name: string;
  program: string;
  args: string[];
}

export type LoadStatus =
  | { kind: "fresh" }
  | { kind: "loaded" }
  | { kind: "recovered"; backup: string };

export interface LoadResult {
  config: Config;
  status: LoadStatus;
}
