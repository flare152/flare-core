#!/bin/bash

# Flare IM 客户端示例演示脚本
# 同时运行服务器和客户端示例

set -e

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
    
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo 未安装"
        exit 1
    fi
    
    if ! command -v openssl &> /dev/null; then
        log_error "OpenSSL 未安装"
        exit 1
    fi
    
    log_success "依赖检查通过"
}

# 生成证书
generate_certs() {
    log_info "检查证书..."
    
    if [ ! -f "certs/server.crt" ] || [ ! -f "certs/server.key" ]; then
        log_info "生成证书..."
        ./scripts/generate_certs.sh
    else
        log_success "证书已存在"
    fi
}

# 编译项目
build_project() {
    log_info "编译项目..."
    cargo build --examples
    log_success "编译完成"
}

# 启动服务器
start_server() {
    log_info "启动服务器..."
    
    # 在后台启动服务器
    cargo run --example server_example > server.log 2>&1 &
    SERVER_PID=$!
    
    # 等待服务器启动
    sleep 3
    
    # 检查服务器是否启动成功
    if kill -0 $SERVER_PID 2>/dev/null; then
        log_success "服务器启动成功 (PID: $SERVER_PID)"
        return 0
    else
        log_error "服务器启动失败"
        return 1
    fi
}

# 运行客户端示例
run_client_example() {
    local example_name=$1
    local description=$2
    
    log_info "运行 $description..."
    
    # 运行客户端示例
    cargo run --example $example_name > ${example_name}.log 2>&1 &
    CLIENT_PID=$!
    
    # 等待客户端运行
    sleep 10
    
    # 检查客户端是否运行成功
    if kill -0 $CLIENT_PID 2>/dev/null; then
        log_success "$description 运行成功 (PID: $CLIENT_PID)"
        return 0
    else
        log_warn "$description 运行失败或已结束"
        return 1
    fi
}

# 停止所有进程
stop_all() {
    log_info "停止所有进程..."
    
    # 停止服务器
    if [ ! -z "$SERVER_PID" ] && kill -0 $SERVER_PID 2>/dev/null; then
        kill $SERVER_PID
        log_info "服务器已停止"
    fi
    
    # 停止客户端
    if [ ! -z "$CLIENT_PID" ] && kill -0 $CLIENT_PID 2>/dev/null; then
        kill $CLIENT_PID
        log_info "客户端已停止"
    fi
    
    # 停止所有相关进程
    pkill -f "server_example" || true
    pkill -f "websocket_client" || true
    pkill -f "quic_client" || true
    pkill -f "auto_racing_client" || true
    
    log_success "所有进程已停止"
}

# 显示日志
show_logs() {
    local example_name=$1
    
    if [ -f "${example_name}.log" ]; then
        echo ""
        log_info "$example_name 日志:"
        echo "=================="
        tail -20 ${example_name}.log
        echo ""
    fi
}

# 清理日志文件
cleanup_logs() {
    log_info "清理日志文件..."
    rm -f server.log websocket_client.log quic_client.log auto_racing_client.log
    log_success "日志文件已清理"
}

# 显示帮助信息
show_help() {
    echo "Flare IM 客户端示例演示脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -h, --help          显示此帮助信息"
    echo "  -s, --server-only   仅启动服务器"
    echo "  -w, --websocket     运行 WebSocket 客户端示例"
    echo "  -q, --quic          运行 QUIC 客户端示例"
    echo "  -a, --auto-racing   运行协议竞速客户端示例"
    echo "  -c, --clean         清理日志文件"
    echo "  -k, --kill          停止所有进程"
    echo ""
    echo "示例:"
    echo "  $0                   # 运行完整演示"
    echo "  $0 -s                # 仅启动服务器"
    echo "  $0 -w                # 运行 WebSocket 客户端"
    echo "  $0 -q                # 运行 QUIC 客户端"
    echo "  $0 -a                # 运行协议竞速客户端"
    echo "  $0 -c                # 清理日志"
    echo "  $0 -k                # 停止所有进程"
}

# 主函数
main() {
    case "${1:-}" in
        -h|--help)
            show_help
            exit 0
            ;;
        -s|--server-only)
            check_dependencies
            generate_certs
            build_project
            start_server
            log_info "服务器正在运行，按 Ctrl+C 停止"
            trap stop_all EXIT
            wait
            ;;
        -w|--websocket)
            check_dependencies
            generate_certs
            build_project
            start_server
            run_client_example "websocket_client" "WebSocket 客户端示例"
            show_logs "websocket_client"
            stop_all
            ;;
        -q|--quic)
            check_dependencies
            generate_certs
            build_project
            start_server
            run_client_example "quic_client" "QUIC 客户端示例"
            show_logs "quic_client"
            stop_all
            ;;
        -a|--auto-racing)
            check_dependencies
            generate_certs
            build_project
            start_server
            run_client_example "auto_racing_client" "协议竞速客户端示例"
            show_logs "auto_racing_client"
            stop_all
            ;;
        -c|--clean)
            cleanup_logs
            exit 0
            ;;
        -k|--kill)
            stop_all
            exit 0
            ;;
        "")
            # 完整演示
            check_dependencies
            generate_certs
            build_project
            
            log_info "开始完整演示..."
            
            # 启动服务器
            start_server
            
            # 运行 WebSocket 客户端
            run_client_example "websocket_client" "WebSocket 客户端示例"
            sleep 5
            
            # 运行 QUIC 客户端
            run_client_example "quic_client" "QUIC 客户端示例"
            sleep 5
            
            # 运行协议竞速客户端
            run_client_example "auto_racing_client" "协议竞速客户端示例"
            sleep 5
            
            # 显示所有日志
            show_logs "websocket_client"
            show_logs "quic_client"
            show_logs "auto_racing_client"
            
            # 停止所有进程
            stop_all
            
            log_success "演示完成！"
            ;;
        *)
            log_error "未知选项: $1"
            show_help
            exit 1
            ;;
    esac
}

# 设置信号处理
trap stop_all EXIT INT TERM

# 运行主函数
main "$@" 