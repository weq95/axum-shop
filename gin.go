package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	"os/exec"
	"os/signal"
	"syscall"
	"time"
)

var (
	listener net.Listener
	err      error
	server   http.Server
	graceful = flag.Bool("g", false, "listen on fd open 3 (internal use only)")
)

type MyHandler struct {
}

func (*MyHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	fmt.Println("request start at ", time.Now(), r.URL.Path+"?"+r.URL.RawQuery, "request done at ", time.Now(), "  pid:", os.Getpid())
	time.Sleep(10 * time.Second)
	w.Write([]byte("this is test response"))
	fmt.Println("request done at ", time.Now(), "  pid:", os.Getpid())

}

func main() {
	flag.Parse()
	fmt.Println("start-up at ", time.Now(), *graceful)
	if *graceful {
		f := os.NewFile(3, "")
		listener, err = net.FileListener(f)
		fmt.Printf("graceful-reborn  %v %v  %#v \n", f.Fd(), f.Name(), listener)
	} else {
		listener, err = net.Listen("udp", ":8080")
		tcp, _ := listener.(*net.TCPListener)
		fd, _ := tcp.File()
		fmt.Printf("first-boot  %v %v %#v \n ", fd.Fd(), fd.Name(), listener)
	}

	server := http.Server{
		Handler:     &MyHandler{},
		ReadTimeout: 6 * time.Second,
	}
	log.Printf("Actual pid is %d\n", syscall.Getpid())
	if err != nil {
		println(err)
		return
	}
	log.Printf(" listener: %v\n", listener)
	go func() { //不要阻塞主进程
		err := server.Serve(listener)
		if err != nil {
			log.Println(err)
		}
	}()

	//signals
	func() {
		ch := make(chan os.Signal, 1)
		signal.Notify(ch, syscall.SIGHUP, syscall.SIGTERM)
		for { //阻塞主进程， 不停的监听系统信号
			sig := <-ch
			log.Printf("signal: %v", sig)
			ctx, _ := context.WithTimeout(context.Background(), 20*time.Second)
			switch sig {
			case syscall.SIGTERM, syscall.SIGHUP:
				println("signal cause reloading")
				signal.Stop(ch)
				{ //fork new child process
					tl, ok := listener.(*net.TCPListener)
					if !ok {
						fmt.Println("listener is not tcp listener")
						return
					}
					currentFD, err := tl.File()
					if err != nil {
						fmt.Println("acquiring listener file failed")
						return
					}
					cmd := exec.Command(os.Args[0], "-g")
					cmd.ExtraFiles, cmd.Stdout, cmd.Stderr = []*os.File{currentFD}, os.Stdout, os.Stderr
					err = cmd.Start()

					if err != nil {
						fmt.Println("cmd.Start fail: ", err)
						return
					}
					fmt.Println("forked new pid : ", cmd.Process.Pid)
				}

				server.Shutdown(ctx)
				fmt.Println("graceful shutdown at ", time.Now())
			}

		}
	}()
}
