export declare const signTransaction: (transactionXdr: string, opts?: {
    network?: string | undefined;
    networkPassphrase?: string | undefined;
    accountToSign?: string | undefined;
} | undefined) => Promise<string>;
