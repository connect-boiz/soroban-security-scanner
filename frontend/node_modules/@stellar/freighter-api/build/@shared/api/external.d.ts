import { UserInfo } from "./types";
export declare const requestPublicKey: () => Promise<string>;
export declare const submitTransaction: (transactionXdr: string, opts?: string | {
    network?: string | undefined;
    accountToSign?: string | undefined;
    networkPassphrase?: string | undefined;
} | undefined, accountToSign?: string | undefined) => Promise<string>;
export declare const submitBlob: (blob: string, opts?: {
    accountToSign?: string | undefined;
} | undefined) => Promise<string>;
export declare const submitAuthEntry: (entryXdr: string, opts?: {
    accountToSign?: string | undefined;
} | undefined) => Promise<string>;
export declare const requestNetwork: () => Promise<string>;
export declare const requestNetworkDetails: () => Promise<{
    network: string;
    networkUrl: string;
    networkPassphrase: string;
    sorobanRpcUrl?: string | undefined;
}>;
export declare const requestConnectionStatus: () => Promise<boolean>;
export declare const requestAllowedStatus: () => Promise<boolean>;
export declare const setAllowedStatus: () => Promise<boolean>;
export declare const requestUserInfo: () => Promise<UserInfo>;
