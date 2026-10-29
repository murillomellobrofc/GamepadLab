export type ButtonStatus = 'pending' | 'pass' | 'fail' | 'skip';
export interface ButtonResult {
  button: number;
  name: string;
  status: ButtonStatus;
  maxValue: number;
}

export function summarize(results: ButtonResult[]) {
  return {
    pass: results.filter((r) => r.status === 'pass').length,
    fail: results.filter((r) => r.status === 'fail').length,
    skip: results.filter((r) => r.status === 'skip').length,
    failed: results.filter((r) => r.status === 'fail').map((r) => r.name),
  };
}
