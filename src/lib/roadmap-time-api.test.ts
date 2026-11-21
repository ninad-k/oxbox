import {expect,it} from 'vitest';
import {countdownText} from './roadmap-time-api';
it('formats future, reached, and elapsed countdown dates',()=>{expect(countdownText(90061)).toBe('1d 1h 1m 1s remaining');expect(countdownText(0)).toBe('0d 0h 0m 0s remaining');expect(countdownText(-65)).toBe('0d 0h 1m 5s ago');});
