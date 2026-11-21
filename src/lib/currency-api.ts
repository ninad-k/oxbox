import {call} from './ipc';
export interface RateBook {date:string;fetchedAt:number;rates:Record<string,number>}
export interface CurrencyStatus {networkEnabled:boolean;networkIssue:string|null;book:RateBook|null}
export const currencyApi={
  currencyStatus:()=>call<CurrencyStatus>('currency_status'),
  currencySetNetwork:(enabled:boolean)=>call<void>('currency_set_network',{enabled}),
  currencyImport:(path:string)=>call<RateBook>('currency_import',{path}),
  currencyRefresh:(force=false)=>call<RateBook>('currency_refresh',{force}),
  currencyConvert:(amount:number,from:string,to:string)=>call<number>('currency_convert',{amount,from,to}),
};
