/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export interface Transformation {
  start: number
  end: number
  replacement: string
}
export interface AnalysisResult {
  hasComponent: boolean
  filePath: string
  dependencies: Array<string>
  transformations: Array<Transformation>
}
export declare function analyzeFile(filePath: string): AnalysisResult
export declare function analyzeFileChanged(filePath: string, event: string): void
export declare function analyzeAndTransformCode(code: string, filePath: string): string
