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
