#!/bin/bash

# 生成 TLS 证书脚本
# 简化方案：服务端证书 + 客户端验证证书

set -e

echo "🔐 生成 TLS 简化证书"
echo "=================="

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查依赖
check_dependencies() {
    log_info "检查依赖..."
    
    if ! command -v openssl &> /dev/null; then
        log_error "OpenSSL 未安装"
        exit 1
    fi
    
    log_success "依赖检查通过"
}

# 创建目录
create_directories() {
    log_info "创建证书目录..."
    
    mkdir -p certs
    
    log_success "目录创建完成"
}

# 生成服务端证书
generate_server_cert() {
    log_info "生成服务端证书..."
    
    cd certs
    
    # 生成服务端私钥
    openssl genrsa -out server.key 2048
    
    # 生成服务端证书签名请求
    openssl req -new -key server.key -out server.csr -subj "/C=CN/ST=Beijing/L=Beijing/O=FlareIM/OU=Server/CN=flare-core-server"
    
    # 创建服务端证书配置文件
    cat > server.conf << EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = CN
ST = Beijing
L = Beijing
O = FlareIM
OU = Server
CN =flare-core-server

[v3_req]
keyUsage = keyEncipherment, dataEncipherment, digitalSignature
extendedKeyUsage = serverAuth, clientAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = *.localhost
DNS.3 = flare-core-server
IP.1 = 127.0.0.1
IP.2 = ::1
EOF
    
    # 自签名服务端证书
    openssl x509 -req -in server.csr -signkey server.key -out server.crt -days 365 -extensions v3_req -extfile server.conf
    
    cd ..
    
    log_success "服务端证书生成完成"
}

# 生成客户端验证证书
generate_client_cert() {
    log_info "生成客户端验证证书..."
    
    cd certs
    
    # 复制服务端证书作为客户端验证证书
    cp server.crt client.crt
    
    cd ..
    
    log_success "客户端验证证书生成完成"
}

# 清理临时文件
cleanup_temp_files() {
    log_info "清理临时文件..."
    
    cd certs
    rm -f server.csr server.conf
    cd ..
    
    log_success "临时文件清理完成"
}

# 设置权限
set_permissions() {
    log_info "设置证书权限..."
    
    chmod 600 certs/*.key
    chmod 644 certs/*.crt
    
    log_success "权限设置完成"
}

# 验证证书
verify_certs() {
    log_info "验证证书..."
    
    # 验证服务端证书
    if openssl x509 -in certs/server.crt -text -noout &> /dev/null; then
        log_success "服务端证书验证通过"
    else
        log_error "服务端证书验证失败"
        exit 1
    fi
    
    # 验证客户端证书
    if openssl x509 -in certs/client.crt -text -noout &> /dev/null; then
        log_success "客户端证书验证通过"
    else
        log_error "客户端证书验证失败"
        exit 1
    fi
}

# 显示证书信息
show_cert_info() {
    log_info "证书信息:"
    echo ""
    echo "📁 证书文件:"
    echo "  certs/server.crt - 服务端证书"
    echo "  certs/server.key - 服务端私钥"
    echo "  certs/client.crt - 客户端验证证书（服务端证书的副本）"
    echo ""
    echo "🔍 证书详情:"
    echo "  服务端证书:"
    openssl x509 -in certs/server.crt -subject -issuer -dates -noout
    echo ""
    echo "  客户端证书:"
    openssl x509 -in certs/client.crt -subject -issuer -dates -noout
    echo ""
    echo "🔗 证书关系:"
    echo "  服务端证书：自签名，用于服务端身份验证"
    echo "  客户端证书：服务端证书的副本，用于验证服务端身份"
}

# 清理临时文件
cleanup() {
    log_info "清理临时文件..."
    
    # 清理临时文件
    cleanup_temp_files
    
    log_success "清理完成"
}

# 主函数
main() {
    case "${1:-}" in
        -h|--help)
            echo "TLS 简化证书生成脚本"
            echo ""
            echo "用法: $0 [选项]"
            echo ""
            echo "选项:"
            echo "  -h, --help     显示此帮助信息"
            echo "  -c, --clean    清理临时文件"
            echo "  -v, --verify   仅验证证书"
            echo "  -i, --info     显示证书信息"
            echo ""
            echo "证书方案:"
            echo "  - 服务端证书：自签名，用于服务端身份验证"
            echo "  - 客户端证书：服务端证书的副本，用于验证服务端身份"
            echo ""
            echo "示例:"
            echo "  $0              # 生成所有证书"
            echo "  $0 -c           # 清理临时文件"
            echo "  $0 -v           # 验证证书"
            echo "  $0 -i           # 显示证书信息"
            exit 0
            ;;
        -c|--clean)
            cleanup
            exit 0
            ;;
        -v|--verify)
            verify_certs
            exit 0
            ;;
        -i|--info)
            show_cert_info
            exit 0
            ;;
        "")
            check_dependencies
            create_directories
            generate_server_cert
            generate_client_cert
            set_permissions
            verify_certs
            show_cert_info
            cleanup
            log_success "简化证书生成完成！"
            ;;
        *)
            log_error "未知选项: $1"
            echo "使用 $0 -h 查看帮助信息"
            exit 1
            ;;
    esac
}

# 运行主函数
main "$@" 