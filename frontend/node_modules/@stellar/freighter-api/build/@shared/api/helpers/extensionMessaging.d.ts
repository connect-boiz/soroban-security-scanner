import { EXTERNAL_SERVICE_TYPES, SERVICE_TYPES } from "../../constants/services";
import { Response } from "../types";
interface Msg {
    [key: string]: any;
    type: EXTERNAL_SERVICE_TYPES | SERVICE_TYPES;
}
export declare const sendMessageToContentScript: (msg: Msg) => Promise<Response>;
export declare const sendMessageToBackground: (msg: Msg) => Promise<Response>;
export {};
