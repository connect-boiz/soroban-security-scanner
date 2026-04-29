export declare const getNetworkDetails: () => Promise<{
    network: string;
    networkUrl: string;
    networkPassphrase: string;
    sorobanRpcUrl?: string;
}>;
