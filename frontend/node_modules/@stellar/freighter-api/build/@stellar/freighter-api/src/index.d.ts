import { getPublicKey } from "./getPublicKey";
import { signTransaction } from "./signTransaction";
import { signBlob } from "./signBlob";
import { signAuthEntry } from "./signAuthEntry";
import { isConnected } from "./isConnected";
import { getNetwork } from "./getNetwork";
import { getNetworkDetails } from "./getNetworkDetails";
import { isAllowed } from "./isAllowed";
import { setAllowed } from "./setAllowed";
import { getUserInfo } from "./getUserInfo";
export declare const isBrowser: boolean;
export { getPublicKey, signTransaction, signBlob, signAuthEntry, isConnected, getNetwork, getNetworkDetails, isAllowed, setAllowed, getUserInfo, };
declare const _default: {
    getPublicKey: () => Promise<string>;
    signTransaction: (transactionXdr: string, opts?: {
        network?: string | undefined;
        networkPassphrase?: string | undefined;
        accountToSign?: string | undefined;
    } | undefined) => Promise<string>;
    signBlob: (blob: string, opts?: {
        accountToSign?: string | undefined;
    } | undefined) => Promise<string>;
    signAuthEntry: (entryXdr: string, opts?: {
        accountToSign?: string | undefined;
    } | undefined) => Promise<string>;
    isConnected: () => Promise<boolean>;
    getNetwork: () => Promise<string>;
    getNetworkDetails: () => Promise<{
        network: string;
        networkUrl: string;
        networkPassphrase: string;
        sorobanRpcUrl?: string | undefined;
    }>;
    isAllowed: () => Promise<boolean>;
    setAllowed: () => Promise<boolean>;
    getUserInfo: () => Promise<{
        publicKey: string;
    }>;
};
export default _default;
