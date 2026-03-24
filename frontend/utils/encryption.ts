import CryptoJS from 'crypto-js';

export class EncryptionService {
  private static generateKey(salt: string, password: string): string {
    return CryptoJS.PBKDF2(password, salt, {
      keySize: 256 / 32,
      iterations: 10000
    }).toString();
  }

  static encrypt(data: string, publicKey: string): { encrypted: string; salt: string } {
    const salt = CryptoJS.lib.WordArray.random(128 / 8).toString();
    const key = this.generateKey(salt, publicKey);
    
    const encrypted = CryptoJS.AES.encrypt(data, key).toString();
    
    return {
      encrypted,
      salt
    };
  }

  static decrypt(encryptedData: string, salt: string, privateKey: string): string {
    try {
      const key = this.generateKey(salt, privateKey);
      const decrypted = CryptoJS.AES.decrypt(encryptedData, key);
      
      return decrypted.toString(CryptoJS.enc.Utf8);
    } catch (error) {
      throw new Error('Failed to decrypt data. Invalid credentials or corrupted data.');
    }
  }

  static generateKeyPair(): { publicKey: string; privateKey: string } {
    // In a real implementation, this would use proper asymmetric encryption
    // For demo purposes, we'll use symmetric encryption with derived keys
    const publicKey = CryptoJS.lib.WordArray.random(256 / 8).toString();
    const privateKey = CryptoJS.lib.WordArray.random(256 / 8).toString();
    
    return { publicKey, privateKey };
  }

  static hash(data: string): string {
    return CryptoJS.SHA256(data).toString();
  }

  static verifyHash(data: string, hash: string): boolean {
    return this.hash(data) === hash;
  }
}
